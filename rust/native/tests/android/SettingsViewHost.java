// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
package moe.meltype.rust;
import android.content.Context;
import android.view.View;
import android.widget.*;
import java.nio.file.*;
import java.util.*;
public final class SettingsViewHost {
    final Context context=new Context();final LinearLayout body=new LinearLayout(context);
    final List<String> actions=new ArrayList<>();boolean failCallback;
    private View.OnClickListener settingsButtonCallback(String operation){if(failCallback)throw new IllegalStateException("listener failed");return view->actions.add(operation);}
    private CompoundButton.OnCheckedChangeListener settingsToggleCallback(String operation){return (view,checked)->actions.add(operation);}
    private AdapterView.OnItemSelectedListener settingsSelectCallback(String operation){return new AdapterView.OnItemSelectedListener(){
        public void onNothingSelected(AdapterView parent){}
        public void onItemSelected(AdapterView parent,View view,int position,long id){actions.add(operation);}
    };}
    static String ok(String reply){if(reply==null||reply.contains("\"error\":"))throw new AssertionError(reply);return reply;}
    static void check(boolean value,Object detail){if(!value)throw new AssertionError(detail);}
    EditText field(String hint){for(View view:body.children)if(view instanceof EditText edit&&edit.hint.equals(hint))return edit;throw new AssertionError(hint);}
    public static void main(String[] args)throws Exception {
        Path profile=Files.createTempDirectory("meltype-jni-settings-view-");long session=Native.create(profile.toString());
        try {
            SettingsViewHost host=new SettingsViewHost();
            ok(Native.renderSettingsPage(host,"{\"nodes\":[{\"type\":\"button\",\"text\":\"インストール済み\",\"enabled\":false,\"action\":{}},{\"type\":\"button\",\"text\":\"導入する\",\"action\":{}}]}"));
            check(!host.body.children.get(0).enabled&&host.body.children.get(1).enabled,"installed button disabled; legacy buttons remain enabled");
            for(int page=0;page<6;page++)ok(Native.renderSettingsPage(host,Native.settingsPage(session,page)));
            ok(Native.renderSettingsPage(host,Native.settingsPage(session,0)));check(host.actions.isEmpty(),"initial state cannot mutate settings");
            int toggles=0;for(View view:host.body.children)if(view instanceof Switch toggle){toggles++;toggle.performClick();}
            check(toggles==14&&host.actions.size()==14,"all switch callbacks");String operation=host.actions.get(0);
            check(ok(Native.settingsValues(host,operation)).contains("\"checked\":"),"typed switch value");
            check(!ok(Native.settingsValues(host,"{\"action\":\"other\"}")).contains("\"checked\":"),"unrelated operation cannot read toggle value");
            ok(Native.renderSettingsPage(host,Native.settingsPage(session,2)));check(host.field("展開する文字列").lines==2,"multiline field");
            host.field("呼び出し名").setText("mail");host.field("展開する文字列").setText("一行目\n\"二行目\"😀");
            String values=ok(Native.settingsValues(host,"null"));check(values.contains("mail")&&values.contains("\\n\\\"二行目\\\"😀"),"Unicode/newline field read: "+values);
            host.actions.clear();for(View view:host.body.children)if(view instanceof Button button&&button.text.equals("保存"))button.performClick();
            check(host.actions.size()==1,"save listener");ok(Native.settingsAction(session,host.actions.get(0),values));
            check(Native.control(session,"{\"op\":\"snippets_list\"}").contains("mail"),"renderer-to-core save");
            ok(Native.editSettingsFields(host,"{\"name\":\"changed\",\"text\":\"新しい値😀\"}"));check(host.field("呼び出し名").text.equals("changed"),"edit applied");
            String missing=Native.editSettingsFields(host,"{\"name\":\"must-not-change\",\"missing\":\"x\"}");check(missing.contains("\"error\":"),"missing field rejected");check(host.field("呼び出し名").text.equals("changed"),"missing field cannot partly edit");
            int count=host.body.children.size();for(String invalid:new String[]{"broken","{\"nodes\":[{\"type\":\"bad\"}]}","{\"nodes\":[{\"type\":\"field\",\"id\":\"x\",\"hint\":\"\",\"value\":\"\",\"lines\":0}]}"}) {
                check(Native.renderSettingsPage(host,invalid).contains("\"error\":"),"invalid page rejected");check(host.body.children.size()==count,"validate before clear");
            }
            host.failCallback=true;check(Native.renderSettingsPage(host,Native.settingsPage(session,2)).contains("\"error\":"),"JNI listener error surfaced");host.failCallback=false;
            for(int i=0;i<20;i++)ok(Native.renderSettingsPage(host,Native.settingsPage(session,i%6)));
            StringBuilder many=new StringBuilder("{\"nodes\":[");for(int i=0;i<800;i++){if(i>0)many.append(',');many.append("{\"type\":\"field\",\"id\":\"f").append(i).append("\",\"hint\":\"h\",\"value\":\"v\",\"lines\":1}");}many.append("]}");
            ok(Native.renderSettingsPage(host,many.toString()));check(host.body.children.size()==800,"large page refs");
            check(ok(Native.settingsValues(host,"null")).contains("\"f799\":\"v\""),"all fields read");ok(Native.editSettingsFields(host,"{\"f799\":\"last\"}"));
            check(ok(Native.settingsValues(host,"null")).contains("\"f799\":\"last\""),"large page edit refs");
            System.out.println("Android JNI settings widgets: six native pages, 14 switches/callbacks, Unicode field/save/edit paths, prevalidation/failure recovery, 20 rebuilds and 800-field local refs passed (SDK-shaped host)");
        } finally {
            Native.destroy(session);
            try(var paths=Files.walk(profile)){for(Path path:paths.sorted(java.util.Comparator.reverseOrder()).toList())Files.delete(path);}
        }
    }
}
