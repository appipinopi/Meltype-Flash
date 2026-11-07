// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
package moe.meltype.rust;
import android.content.Context;
import android.view.View;
import android.widget.*;
/** SDK-shaped listener bridge for the native orchestrator host. */
final class SettingsPage {
    final Context context;final LinearLayout body;final SettingsUiHost owner;
    SettingsPage(SettingsUiHost owner){this.owner=owner;context=owner;body=owner.body;}
    private View.OnClickListener settingsButtonCallback(String operation){return view->owner.last=Native.settingsUiAction(owner.session,owner,operation);}
    private CompoundButton.OnCheckedChangeListener settingsToggleCallback(String operation){return (view,checked)->owner.last=Native.settingsUiAction(owner.session,owner,operation);}
    private AdapterView.OnItemSelectedListener settingsSelectCallback(String operation){return new AdapterView.OnItemSelectedListener(){
        public void onNothingSelected(AdapterView parent){}
        public void onItemSelected(AdapterView parent,View view,int position,long id){owner.last=Native.settingsUiAction(owner.session,owner,operation.substring(0,operation.length()-1)+",\"value\":"+position+"}");}
    };}
}
