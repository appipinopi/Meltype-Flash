// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 藤島
package android.view;
/** SDK-shaped gesture data; real MotionEvent coverage lives in androidTest. */
public final class MotionEvent {
    public static final int ACTION_DOWN=0,ACTION_UP=1,ACTION_MOVE=2,ACTION_CANCEL=3;
    private final int action,count,id;
    private final float x,y;
    public MotionEvent(int action,float x,float y){this(action,x,y,1,0);}
    public MotionEvent(int action,float x,float y,int count,int id){this.action=action;this.x=x;this.y=y;this.count=count;this.id=id;}
    public int getActionMasked(){return action;}
    public int getPointerCount(){return count;}
    public int getPointerId(int index){return id;}
    public float getX(){return x;}
    public float getY(){return y;}
}
