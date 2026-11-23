// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
package moe.meltype.rust;
import android.content.Context;
import android.view.View;
import android.widget.*;
import java.util.*;
public final class ActivityViewHost extends Context {
    LinearLayout body;TextView status;View content;
    boolean ready,failReady;final List<Integer> selections=new ArrayList<>();
    private void settingsFormReady(){check(body!=null&&status!=null,"fields before initialization");if(failReady)throw new IllegalStateException("form initialization failed");ready=true;}
    private AdapterView.OnItemSelectedListener settingsNavigationCallback(){check(ready,"form before listener");return new AdapterView.OnItemSelectedListener(){
        public void onNothingSelected(AdapterView parent){}
        public void onItemSelected(AdapterView parent,View view,int position,long id){check(ready&&body!=null,"initialized navigation");selections.add(position);}
    };}
    public void setContentView(View content){check(ready,"form initialized before attachment");this.content=content;((Spinner)((LinearLayout)((ScrollView)content).children.get(0)).children.get(2)).choose(0);}
    static void check(boolean value,Object detail){if(!value)throw new AssertionError(detail);}
    public static void main(String[] args){
        ActivityViewHost activity=new ActivityViewHost();String reply=Native.createSettingsActivity(activity);check(reply.equals("{}"),reply);
        check(activity.content instanceof ScrollView,"scroll root");LinearLayout root=(LinearLayout)((ScrollView)activity.content).children.get(0);
        check(root.orientation==1&&Arrays.equals(root.padding,new int[]{44,44,44,44}),"16dp padding and vertical layout");
        check(root.children.size()==4,"title,status,navigation,body order");TextView title=(TextView)root.children.get(0);
        check(title.text.equals("Meltype-FLASH 設定")&&title.textSize==24,"existing title");check(root.children.get(1)==activity.status&&root.children.get(3)==activity.body,"published fields");
        Spinner navigation=(Spinner)root.children.get(2);ArrayAdapter adapter=(ArrayAdapter)navigation.adapter;
        check(adapter.layout==android.R.layout.simple_spinner_dropdown_item,"framework resource");
        check(Arrays.equals(adapter.labels,new String[]{"入力設定","ユーザー辞書","定型文","学習","プロファイル","バックアップ・診断"}),"page labels/order");
        check(activity.selections.equals(List.of(0)),"attachment selection sees ready form");for(int i=1;i<6;i++)navigation.choose(i);
        check(activity.selections.equals(List.of(0,1,2,3,4,5)),"navigation callbacks");
        ActivityViewHost failed=new ActivityViewHost();failed.failReady=true;check(Native.createSettingsActivity(failed).contains("\"error\":"),"JNI initialization error");check(failed.content==null,"failed initialization never attaches");
        failed.failReady=false;check(Native.createSettingsActivity(failed).equals("{}"),"rebuild after failure");
        for(int i=0;i<20;i++)check(Native.createSettingsActivity(new ActivityViewHost()).equals("{}"),"repeated activity construction");
        System.out.println("Android JNI activity frame: hierarchy, 16dp padding, title, six page labels, field publication/listener/attachment order, initialization failure recovery and 20 builds passed (SDK-shaped host)");
    }
}
