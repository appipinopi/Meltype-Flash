// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
package moe.meltype.rust;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;

/** Actual JVM/JNI calls with a controlled InputConnection-shaped editor. */
public final class EditorHost {
    public static final class Editor implements android.view.inputmethod.InputConnection {
        String before="",selected="",pending="",after="";
        boolean reject,throwsCommit,throwsRead,unsupported;
        boolean throwsFinish;
        Runnable onFinish,onCommit,onRead,onSelected;
        final List<String> calls=new ArrayList<>();
        public CharSequence getSelectedText(int flags) {calls.add("selected");if(onSelected!=null){Runnable callback=onSelected;onSelected=null;callback.run();}return selected;}
        public CharSequence getTextBeforeCursor(int count,int flags) {calls.add("before:"+count);if(onRead!=null){Runnable callback=onRead;onRead=null;callback.run();}if(throwsRead)throw new IllegalStateException("test read failure");if(unsupported)return null;return before.substring(Math.max(0,before.length()-count));}
        public CharSequence getTextAfterCursor(int count,int flags){calls.add("after:"+count);return after.substring(0,Math.min(count,after.length()));}
        public boolean beginBatchEdit(){calls.add("begin");return false;}
        public boolean endBatchEdit(){calls.add("end");return false;}
        public boolean setComposingText(CharSequence text,int cursor){calls.add("compose:"+text);if(reject)return false; before=before.substring(0,before.length()-pending.length())+text;pending=text.toString();return true;}
        public boolean deleteSurroundingText(int count,int after){calls.add("delete:"+count);if(reject)return false;before=before.substring(0,before.length()-count);return true;}
        public boolean commitText(CharSequence text,int cursor){calls.add("commit:"+text);if(throwsCommit)throw new IllegalStateException("test editor failure");if(reject)return false;before=before.substring(0,before.length()-pending.length())+text;pending="";selected="";if(onCommit!=null){Runnable callback=onCommit;onCommit=null;callback.run();}return true;}
        public boolean finishComposingText(){calls.add("finish");if(throwsFinish)throw new IllegalStateException("test finish failure");if(onFinish!=null){Runnable callback=onFinish;onFinish=null;callback.run();}pending="";return !reject;}
        public boolean deleteSurroundingTextInCodePoints(int count,int afterCount){calls.add("codepoints:"+count);if(reject)return false;before=before.substring(0,before.offsetByCodePoints(before.length(),-Math.min(count,before.codePointCount(0,before.length()))));return true;}
        public boolean sendKeyEvent(android.view.KeyEvent event){
            calls.add("event:"+event.action+":"+event.keyCode);if(reject)return false;
            if(event.action==0&&event.keyCode==21&&!before.isEmpty()){int offset=before.offsetByCodePoints(before.length(),-1);after=before.substring(offset)+after;before=before.substring(0,offset);}
            if(event.action==0&&event.keyCode==22&&!after.isEmpty()){int offset=after.offsetByCodePoints(0,1);before+=after.substring(0,offset);after=after.substring(offset);}
            return true;
        }
        public boolean performEditorAction(int action){calls.add("action:"+action);return !reject;}
    }
    static void check(boolean condition,Object detail){if(!condition)throw new AssertionError(detail);}
    static String reply(String edits,String view){return "{\"consumed\":true,\"commits\":"+edits+",\"view\":"+view+"}";}
    static String apply(long session,Editor editor,String json){return Native.applyEditor(session,editor,json,editor.pending);}
    public static void main(String[] args)throws Exception {
        Path profile=Files.createTempDirectory("meltype-jni-editor-");
        long session=Native.create(profile.toString());check(session!=0,"session");
        try {
            Editor e=new Editor();e.before="前😀かな";e.pending="かな";
            String result=apply(session,e,reply("[{\"deleteBefore\":2,\"expect\":\"😀\",\"text\":\"橋\"}]","null"));
            check(e.before.equals("前橋"),e.before);check(result.contains("\"consumed\":true"),result);
            check(e.calls.equals(List.of("begin","selected","before:4","compose:","delete:2","commit:橋","finish","end")),e.calls);
            e=new Editor();e.before="異なる";
            result=apply(session,e,reply("[{\"deleteBefore\":1,\"expect\":\"前\",\"text\":\"橋\"}]","null"));
            check(e.before.equals("異なる")&&e.calls.get(e.calls.size()-1).equals("end"),e.calls);
            check(result.contains("\"consumed\":false"),result);
            e=new Editor();e.before="\ud800";
            apply(session,e,reply("[{\"deleteBefore\":1,\"expect\":\"�\",\"text\":\"x\"}]","null"));
            check(e.before.equals("\ud800"),"lone surrogate must not match replacement character");
            e=new Editor();e.before="前";e.selected="選択";
            apply(session,e,reply("[{\"deleteBefore\":1,\"expect\":\"前\",\"text\":\"x\"}]","null"));
            check(e.before.equals("前"),"nonempty selection blocks prefix deletion");
            e=new Editor();e.selected="別";
            apply(session,e,"{\"consumed\":true,\"commits\":[],\"view\":null,\"selectionExpect\":\"選択\"}");
            check(!e.calls.contains("begin"),e.calls);
            e=new Editor();e.throwsCommit=true;
            result=apply(session,e,reply("[{\"text\":\"x\"}]","null"));
            check(e.calls.equals(List.of("begin","commit:x","finish","end")),e.calls);
            check(result.contains("\"consumed\":true"),result);
            e=new Editor();e.reject=true;
            apply(session,e,reply("[{\"text\":\"x\"}]","null"));
            check(e.before.isEmpty()&&e.calls.get(e.calls.size()-1).equals("end"),e.calls);
            e=new Editor();
            apply(session,e,reply("[{\"text\":\"x\"}]","{\"text\":4,\"candidates\":[]}"));
            check(e.calls.isEmpty(),"malformed view must be rejected before editing");
            e=new Editor();e.before="前";
            result=apply(session,e,reply("[{\"deleteBefore\":1,\"expect\":\"前\",\"text\":\"橋\"},{\"deleteBefore\":1,\"expect\":\"違\",\"text\":\"x\"}]","null"));
            check(e.before.equals("橋")&&result.contains("\"consumed\":true"),result);
            e=new Editor();
            result=apply(session,e,reply("[]","{\"text\":\"かな😀\",\"candidates\":[\"仮名\"],\"predictions\":[\"かなしい\"]}"));
            check(e.pending.equals("かな😀")&&result.contains("\"composing\":true"),result);
            // Drive real conversion results through the same production JNI
            // route, rather than validating only hand-written reply objects.
            Native.reset(session);Native.direct(session,false);e=new Editor();
            for(int scalar:"konnnichiha".codePoints().toArray()) {
                result=apply(session,e,Native.key(session,0,scalar,0));
                check(result.contains("\"consumed\":true"),result);
            }
            apply(session,e,Native.commit(session));
            check(e.before.equals("こんにちは")&&e.pending.isEmpty(),e.before);
            for(int i=0;i<100;i++){e=new Editor();apply(session,e,reply("[]","null"));check(e.calls.equals(List.of("begin","finish","end")),e.calls);}
            System.out.println("Android JNI editor: UTF-16 guards, selection, ordered edits, composition, rejection, exception/batch cleanup, partial edits and repeated calls passed");
        } finally {
            Native.destroy(session);
            // Remove only this freshly-created test profile, children first.
            try(var paths=Files.walk(profile)){for(Path path:paths.sorted(java.util.Comparator.reverseOrder()).toList())Files.delete(path);}
        }
    }
}
