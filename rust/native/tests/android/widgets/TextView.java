// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
package android.widget;
import android.content.Context;
import android.view.View;
public class TextView extends View {
    public String text="";
    public float textSize=14;
    public int minWidth=64;
    public boolean selectable;
    public void setTextIsSelectable(boolean value){selectable=value;}
    public TextView(Context context){super(context);}
    public void setText(CharSequence value){text=value.toString();}
    public void setTextSize(float value){textSize=value;}
    public void setMinWidth(int value){minWidth=value;}
    public int minHeight;
    public void setMinHeight(int value){minHeight=value;}
}
