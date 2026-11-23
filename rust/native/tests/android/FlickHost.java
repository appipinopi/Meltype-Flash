// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 藤島
package moe.meltype.rust;
import android.view.*;
import android.widget.*;
import java.nio.file.*;

/** Production gesture handler -> JNI -> core -> editor, plus Android regressions. */
public final class FlickHost {
    static void check(boolean value,Object detail){if(!value)throw new AssertionError(detail);}
    static void gesture(Button key,float dx,float dy) {
        key.dispatchTouchEvent(new MotionEvent(MotionEvent.ACTION_DOWN,100,100));
        key.dispatchTouchEvent(new MotionEvent(MotionEvent.ACTION_MOVE,100+dx,100+dy));
        key.dispatchTouchEvent(new MotionEvent(MotionEvent.ACTION_UP,100+dx,100+dy));
    }
    public static void main(String[] args)throws Exception {
        Path profile=Files.createTempDirectory("meltype-flick-regression-");
        Files.writeString(profile.resolve("config.json"),"{\"DetectionLevel\":\"Manual\",\"LiveConversion\":false,\"CursorIntoBrackets\":true,\"FullWidthParentheses\":false}");
        ServiceHost host=new ServiceHost();host.session=Native.create(profile.toString());
        try {
            host.fresh(1);host.root=(LinearLayout)Native.serviceView(host.session,host);
            boolean[] saved={true};
            FlickKeyboard keyboard=FlickKeyboard.attach(host,host.root,new FlickKeyboard.Listener(){
                public void key(int scalar){Native.serviceClick(host.session,host,0,scalar);}
                public void variant(int kind){Native.serviceClick(host.session,host,12,kind);}
                public void layout(boolean flick){saved[0]=flick;}
            },true);
            LinearLayout panel=(LinearLayout)host.root.getChildAt(2);
            Button a=(Button)((LinearLayout)panel.getChildAt(0)).getChildAt(0);
            float[][] moves={{0,0},{-90,0},{0,-90},{90,0},{0,90}};
            for(int group=0;group<12;group++) {
                if(group==9)continue;
                Button key=(Button)((LinearLayout)panel.getChildAt(group/3)).getChildAt(group%3);
                for(int direction=0;direction<5;direction++) {
                    host.fresh(1);gesture(key,moves[direction][0],moves[direction][1]);
                    char expected=FlickKeyboard.GROUPS[group].charAt(direction);
                    check(host.editor.before.equals(String.valueOf(expected)),"group="+group+" direction="+direction+" actual="+host.editor.before);
                    check(Native.composingText(host.session).equals(host.editor.pending),"native state follows flick");
                }
            }
            host.fresh(1);gesture(a,0,0);gesture(a,0,0);check(host.editor.before.equals("ああ"),"repeated tap is not multi-tap cycling");
            host.fresh(1);a.dispatchTouchEvent(new MotionEvent(0,100,100));a.dispatchTouchEvent(new MotionEvent(3,0,0));a.dispatchTouchEvent(new MotionEvent(1,0,0));check(host.editor.before.isEmpty(),"cancel does not input");
            a.dispatchTouchEvent(new MotionEvent(0,100,100));a.dispatchTouchEvent(new MotionEvent(2,100,100,2,0));a.dispatchTouchEvent(new MotionEvent(1,100,100));check(host.editor.before.isEmpty(),"multi-touch cancels");
            a.dispatchTouchEvent(new MotionEvent(0,100,100));keyboard.cancelGestures();a.dispatchTouchEvent(new MotionEvent(1,100,100));check(host.editor.before.isEmpty(),"field change cancels old gesture");
            gesture(a,0,0);Native.serviceClick(host.session,host,3,8);check(host.editor.before.isEmpty()&&host.editor.pending.isEmpty(),"last kana backspace removes text");
            gesture(a,0,0);Native.serviceClick(host.session,host,3,27);check(host.editor.before.isEmpty(),"Esc removes preedit");
            Button variant=(Button)((LinearLayout)panel.getChildAt(3)).getChildAt(0);
            host.fresh(1);Native.serviceClick(host.session,host,0,'は');
            for(String expected:new String[]{"ば","ぱ","は"}){variant.performClick();check(host.editor.before.equals(expected),"voicing: "+host.editor.before);}
            host.fresh(1);Native.serviceClick(host.session,host,0,'つ');variant.performClick();check(host.editor.before.equals("っ"),"small tsu");
            host.fresh(1);Native.serviceClick(host.session,host,0,'や');variant.performClick();check(host.editor.before.equals("ゃ"),"small ya");
            host.fresh(1);for(char c:"にほんご".toCharArray())Native.serviceClick(host.session,host,0,c);
            Native.serviceClick(host.session,host,3,32);check(!host.candidates.children.isEmpty(),"kana enters native conversion");
            ((Button)host.candidates.children.get(0)).performClick();check(host.editor.pending.isEmpty()&&!host.editor.before.isEmpty(),"candidate click commits");
            host.fresh(1);host.type("ka");Native.serviceRestart(host.session,host,host.info);host.type("na");check(host.editor.before.equals("かな"),"restart keeps matching preedit: "+host.editor.before);
            host.fresh(1);host.type("ka");host.editor.before="変更";host.editor.pending="";Native.serviceRestart(host.session,host,host.info);host.type("na");check(host.editor.before.equals("変更な"),"restart respects changed editor text");
            host.fresh(1);host.type("ka");host.info.inputType=0x81;host.editor.calls.clear();Native.serviceRestart(host.session,host,host.info);check(Native.editorFlags(host.session)==3,"restart into password resets policy");check(host.editor.before.equals("か")&&host.editor.calls.equals(java.util.List.of("finish")),"private restart has no reads");
            host.fresh(1);host.type("ka");host.onConnection=()->host.fresh(0x81);Native.serviceRestart(host.session,host,host.info);check(Native.editorFlags(host.session)==3&&host.editor.before.isEmpty(),"restart connection getter must not overwrite new field");
            host.fresh(1);host.type("ka");host.editor.onRead=()->host.fresh(0x81);Native.serviceRestart(host.session,host,host.info);check(Native.editorFlags(host.session)==3&&host.editor.before.isEmpty(),"restart text getter must not overwrite new field");
            host.fresh(0x81);host.editor.before="前";host.editor.selected="選択";Native.serviceClick(host.session,host,3,8);check(host.editor.before.equals("前")&&host.editor.selected.isEmpty(),"backspace deletes selection, preserves prefix");
            host.editor.before="前😀";Native.serviceClick(host.session,host,3,8);check(host.editor.before.equals("前"),"backspace deletes one code point");
            host.fresh(1);Native.serviceClick(host.session,host,0,'(');Native.serviceClick(host.session,host,0,')');Native.serviceClick(host.session,host,3,13);check(host.editor.before.equals("(")&&host.editor.after.equals(")"),"bracket caret moves inside");
            host.fresh(1);Native.applyEditor(host.session,host.editor,"{\"consumed\":true,\"commits\":[{\"text\":\"x\",\"caretBack\":2}],\"view\":null}","");check(host.editor.calls.isEmpty(),"invalid caret movement is rejected before writing");
            Button toggle=(Button)host.root.getChildAt(host.root.getChildCount()-1);
            toggle.performClick();check(!saved[0]&&panel.visibility==View.GONE&&host.root.getChildAt(3).visibility==View.VISIBLE,"QWERTY toggle");
            toggle.performClick();check(saved[0]&&panel.visibility==View.VISIBLE&&host.root.getChildAt(3).visibility==View.GONE,"flick toggle");
            System.out.println("Android flick/regressions: 55 directions, repeat/cancel/multitouch, lifecycle cancellation, variants, conversion/candidate commit, selection/Unicode deletion, Esc, restart and bracket caret passed");
        } finally {
            Native.destroy(host.session);
            try(var paths=Files.walk(profile)){for(Path path:paths.sorted(java.util.Comparator.reverseOrder()).toList())Files.delete(path);}
        }
    }
}
