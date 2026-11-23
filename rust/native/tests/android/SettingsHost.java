// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
package moe.meltype.rust;
import java.nio.file.Files;
import java.nio.file.Path;

/** Actual JNI/core settings contract; framework rendering is tested on Android. */
public final class SettingsHost {
    static void check(boolean value,Object detail){if(!value)throw new AssertionError(detail);}
    static String action(long session,String operation,String fields) {
        String reply=Native.settingsAction(session,operation,fields);
        check(reply!=null&&!reply.contains("\"error\":"),reply);return reply;
    }
    public static void main(String[] args)throws Exception {
        Path root=Files.createTempDirectory("meltype-jni-settings-");long session=Native.create(root.toString());
        try {
            check(session!=0,"session");
            for(int page=0;page<6;page++) {
                String result=Native.settingsPage(session,page);check(result.contains("\"nodes\":[")&&!result.contains("\"error\":"),result);
            }
            String page=Native.settingsPage(session,0);check(page.split("\"type\":\"toggle\"",-1).length-1==13,"13 existing controls");
            check(page.contains("入力を有効にする")&&page.contains("定型文の開始記号"),page);
            action(session,"{\"action\":\"ai_select\",\"value\":1}","{}");
            page=Native.settingsPage(session,0);
            check(page.contains("shuntelligence v7 / S8P")&&page.contains("shuntelligence v5 / XS5"),"version catalog visible");
            check(page.contains("このモデルをインストール")&&!page.contains("インストール済み"),"missing model is not installed");
            action(session,"{\"action\":\"ai_variant\",\"value\":1}","{}");
            String ai=Native.control(session,"{\"op\":\"settings_get\"}");
            check(ai.contains("\"ShuntiModelVariant\":\"shunti-v7-xs5\"")&&ai.contains("\"UseShunti\":false"),"version saved without download or activation");
            action(session,"{\"action\":\"ai_select\",\"value\":0}","{}");
            check(!Native.settingsPage(session,0).contains("このモデルをインストール"),"none hides version installer");
            action(session,"{\"action\":\"ai_select\",\"value\":1}","{}");
            check(Native.control(session,"{\"op\":\"settings_get\"}").contains("shunti-v7-xs5"),"selection retains version");
            action(session,"{\"action\":\"ai_select\",\"value\":0}","{}");
            action(session,"{\"action\":\"toggle\",\"name\":\"PredictiveCandidates\"}","{\"checked\":false}");
            check(Native.control(session,"{\"op\":\"settings_get\"}").contains("\"PredictiveCandidates\":false"),"toggle persisted");
            action(session,"{\"action\":\"dictionary_add\"}","{\"reading\":\"めるたいぷ\",\"word\":\"Meltype😀\"}");
            check(Native.settingsPage(session,1).contains("めるたいぷ → Meltype😀"),"dictionary page");
            action(session,"{\"action\":\"snippet_save\"}","{\"name\":\"mail\",\"text\":\"一行目\\n\\\"二行目\\\"😀\"}");
            long reopened=Native.create(root.toString());
            try{check(Native.control(reopened,"{\"op\":\"snippets_list\"}").contains("二行目"),"separate session sees saved snippet");}finally{Native.destroy(reopened);}
            check(action(session,"{\"action\":\"learning_confirm\"}","{}").contains("学習した内容をすべて削除しますか？"),"confirmation precedes clear");
            check(action(session,"{\"action\":\"pick\",\"operation\":\"backup_restore\"}","{}").contains("\"output\":false"),"restore opens input document");
            for(String input:new String[]{"broken","{\"action\":\"dictionary_add\"}","{\"action\":\"unknown\"}"})
                check(Native.settingsAction(session,input,"{}").contains("\"error\":"),"invalid operation rejected");
            check(Native.settingsPage(session,6).contains("\"error\":"),"invalid page rejected");
            System.out.println("Android JNI settings: six page models, 13 controls, Unicode actions, separate-session persistence, confirmation/document plans and malformed input passed (framework rendering tested separately)");
        } finally {
            Native.destroy(session);
            try(var paths=Files.walk(root)){for(Path path:paths.sorted(java.util.Comparator.reverseOrder()).toList())Files.delete(path);}
        }
    }
}
