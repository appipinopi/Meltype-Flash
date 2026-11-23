// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
package android.view;
import android.content.Context;
import java.util.ArrayList;
import java.util.List;
public class ViewGroup extends View {
    public static class LayoutParams {
        public int width,height;
        public LayoutParams(int width,int height){this.width=width;this.height=height;}
    }
    public final List<View> children=new ArrayList<>();
    public ViewGroup(Context context){super(context);}
    public void addView(View child){children.add(child);}
    public void addView(View child,int index){children.add(index,child);}
    public void addView(View child,LayoutParams params){child.layoutParams=params;addView(child);}
    public int getChildCount(){return children.size();}
    public View getChildAt(int index){return children.get(index);}
    public void removeAllViews(){children.clear();}
}
