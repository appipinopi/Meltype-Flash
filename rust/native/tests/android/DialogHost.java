// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
package moe.meltype.rust;
import android.app.AlertDialog;
import android.content.*;
import android.net.Uri;
import java.io.*;
import java.nio.file.*;
public final class DialogHost extends Context {
    final DocumentHost documents;long session;String reply,payload;boolean accepted;
    DialogHost(Path cache){documents=new DocumentHost(cache);}
    public File getCacheDir(){return documents.getCacheDir();}
    public ContentResolver getContentResolver(){return documents.getContentResolver();}
    private DialogInterface.OnClickListener settingsConfirmationClick(String payload,boolean accept){return (dialog,which)->{this.payload=payload;accepted=accept;reply=Native.completeSettingsConfirmation(session,payload,accept);};}
    private DialogInterface.OnCancelListener settingsConfirmationCancel(String payload){return dialog->{this.payload=payload;accepted=false;reply=Native.completeSettingsConfirmation(session,payload,false);};}
    static void check(boolean value,Object detail){if(!value)throw new AssertionError(detail);}
    static String ok(String reply){check(reply!=null&&!reply.contains("\"error\":"),reply);return reply;}
    void show(String confirmation){ok(Native.showSettingsConfirmation(session,this,confirmation));check(AlertDialog.last.shown,"shown");}
    public static void main(String[] args)throws Exception {
        Path root=Files.createTempDirectory("meltype-jni-dialog-");Path profile=Files.createDirectory(root.resolve("profile"));
        DialogHost host=new DialogHost(Files.createDirectory(root.resolve("cache")));host.session=Native.create(profile.toString());
        try {
            ok(Native.settingsAction(host.session,"{\"action\":\"ai_select\",\"value\":1}","{}"));
            String ai=ok(Native.settingsAction(host.session,"{\"action\":\"ai_install\"}","{}"));
            String confirmationAI=ai.substring("{\"confirm\":".length(),ai.length()-1);
            host.show(confirmationAI);check(AlertDialog.last.title.contains("shunti"),"AI consent names the selected model");
            AlertDialog.last.no.onClick(null,-2);check(ok(host.reply).equals("{\"installShunti\":false}"),"refusal cannot start model download");
            check(Native.control(host.session,"{\"op\":\"settings_get\"}").contains("\"UseShunti\":false"),"selection alone cannot activate AI");
            host.show(confirmationAI);AlertDialog.last.yes.onClick(null,-1);
            check(ok(host.reply).contains("\"installShunti\":true"),"acceptance returns background installation instruction");
            ok(Native.settingsAction(host.session,"{\"action\":\"ai_select\",\"value\":0}","{}"));
            check(!Native.settingsPage(host.session,0).contains("shuntiをインストール"),"no model hides installer");
            String learning="{\"message\":\"学習した内容をすべて削除しますか？\",\"positive\":\"削除\",\"action\":{\"action\":\"learning_clear\"}}";
            host.show(learning);check(AlertDialog.last.title==null&&AlertDialog.last.message.equals("学習した内容をすべて削除しますか？"),"learning title/message");
            check(AlertDialog.last.positive.equals("削除")&&AlertDialog.last.negative.equals("キャンセル"),"labels");
            String before=Native.control(host.session,"{\"op\":\"learning_list\"}");AlertDialog.last.no.onClick(null,-2);check(!host.accepted&&ok(host.reply).equals("{}"),"negative callback");check(before.equals(Native.control(host.session,"{\"op\":\"learning_list\"}")),"cancel does not mutate");
            host.show(learning);AlertDialog.last.cancel.onCancel(null);check(!host.accepted&&ok(host.reply).equals("{}"),"system cancel");
            host.show(learning);AlertDialog.last.yes.onClick(null,-1);check(host.accepted&&ok(host.reply).contains("\"page\":3"),"positive reaches clear/reload");
            host.documents.resolver.output=new DocumentHost.Output();ok(Native.settingsDocument(host.session,host,new Uri(),"backup_create"));
            byte[] backup=((DocumentHost.Output)host.documents.resolver.output).toByteArray();
            for(int mode=0;mode<4;mode++) {
                host.documents.resolver.input=new DocumentHost.Input(backup);String result=ok(Native.settingsDocument(host.session,host,new Uri(),"backup_restore"));
                String confirmation=result.substring("{\"confirm\":".length(),result.length()-1);
                if(mode==3){AlertDialog.failShow=true;check(Native.showSettingsConfirmation(host.session,host,confirmation).contains("\"error\":"),"show error surfaced");AlertDialog.failShow=false;}
                else {
                    host.show(confirmation);check(AlertDialog.last.title.equals("バックアップを復元")&&AlertDialog.last.positive.equals("復元"),"restore title/label");
                    if(mode==0)AlertDialog.last.no.onClick(null,-2);
                    else if(mode==1)AlertDialog.last.cancel.onCancel(null);
                    else AlertDialog.last.yes.onClick(null,-1);
                    ok(host.reply);check(host.accepted==(mode==2),"confirmation disposition");
                    if(mode==2)check(Native.completeSettingsConfirmation(host.session,host.payload,true).contains("\"error\":"),"restore only once");
                }
                host.documents.empty();
            }
            check(Native.showSettingsConfirmation(host.session,host,"{\"message\":\"bad\",\"action\":{\"action\":\"profile_delete\"}}").contains("\"error\":"),"unsupported confirmation rejected");
            check(Native.completeSettingsConfirmation(host.session,"{\"kind\":\"learning\",\"extra\":true}",true).contains("\"error\":"),"malformed callback rejected");
            Native.destroy(host.session);check(Native.completeSettingsConfirmation(host.session,"{\"kind\":\"learning\"}",true).contains("\"error\":"),"destroyed session cannot clear");
            System.out.println("Android JNI confirmations: dialog labels/listeners, learning clear/reload, negative/system cancel, backup restore-once, show failure cleanup and malformed/destroyed callbacks passed (SDK-shaped dialogs)");
        } finally {
            Native.destroy(host.session);
            try(var paths=Files.walk(root)){for(Path path:paths.sorted(java.util.Comparator.reverseOrder()).toList())Files.delete(path);}
        }
    }
}
