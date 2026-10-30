// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
import AppKit
import ObjectiveC

// Intercept only the blocking SDK calls in this private test process. Actual
// NSOpenPanel/NSAlert objects are still allocated/configured by the product.
// Separate real modal tests follow. No system-wide notification is posted.
func testDictionaryDialog(_ editor:NativeEditorPolicy) throws {
    var trace:[String]=[]
    var selected:URL?
    var response=NSApplication.ModalResponse.OK
    var panelState:[String]=[]
    var alertState:[String]=[]
    var notifications:[String]=[]
    func replace(_ cls:AnyClass,_ selector:Selector,_ block:Any)->() -> Void {
        guard let method=class_getInstanceMethod(cls,selector) else {fatalError("missing SDK selector \(selector)")}
        let replacement=imp_implementationWithBlock(block)
        let original=method_setImplementation(method,replacement)
        var restored=false
        return {
            guard !restored else {return}
            restored=true
            method_setImplementation(method,original);imp_removeBlock(replacement)
        }
    }
    func optionalText(_ value:String?)->String {value.map {"value:"+$0} ?? "nil"}
    var checkingNativeChooser = false
    let panelBlock:@convention(block)(AnyObject)->Int = {object in
        guard let panel=object as? NSOpenPanel else {fatalError("not actual NSOpenPanel")}
        if checkingNativeChooser {
            precondition(panel.level == .floating, "#716: chooser must stay above other windows")
            precondition(panel.isVisible, "#716: chooser must be presented before runModal")
        }
        trace.append("choose")
        panelState=[String(panel.allowsMultipleSelection),String(panel.canChooseDirectories),String(panel.canChooseFiles),
            String(panel.resolvesAliases),String(panel.showsHiddenFiles),String(panel.canCreateDirectories),
            optionalText(panel.title),optionalText(panel.prompt),optionalText(panel.nameFieldLabel)]
        return response.rawValue
    }
    let urlBlock:@convention(block)(AnyObject)->NSURL? = {_ in trace.append("url");return selected as NSURL?}
    let alertBlock:@convention(block)(AnyObject)->Int = {object in
        guard let alert=object as? NSAlert else {fatalError("not actual NSAlert")}
        trace.append("alert")
        alertState=[alert.messageText,alert.informativeText,String(alert.alertStyle.rawValue)]
        return NSApplication.ModalResponse.alertFirstButtonReturn.rawValue
    }
    let notifyBlock:@convention(block)(AnyObject,NSString,AnyObject?,NSDictionary?,Bool)->Void = {_,name,object,info,immediate in
        trace.append("notify")
        notifications=[name as String,String(object == nil),String(info == nil),String(immediate)]
    }
    let restorePanel=replace(NSOpenPanel.self,#selector(NSOpenPanel.runModal),panelBlock)
    defer {restorePanel()}
    let restoreURL=replace(NSOpenPanel.self,NSSelectorFromString("URL"),urlBlock)
    defer {restoreURL()}
    let restoreAlert=replace(NSAlert.self,#selector(NSAlert.runModal),alertBlock)
    defer {restoreAlert()}
    let restoreNotify=replace(DistributedNotificationCenter.self,
        NSSelectorFromString("postNotificationName:object:userInfo:deliverImmediately:"),notifyBlock)
    defer {restoreNotify()}
    // Exact former InputController body, excluding its already-tested commit.
    func legacy(_ directory:() -> String?) {
        let panel=NSOpenPanel()
        panel.allowsMultipleSelection=false
        panel.canChooseDirectories=false
        guard panel.runModal() == .OK,let source=panel.url,let folder=directory() else {return}
        let alert=NSAlert()
        do {
            let count=try LegacyMacUserDictionary.importFile(source,directory:folder)
            alert.messageText="ユーザー辞書に\(count)語を追加しました"
            DistributedNotificationCenter.default().postNotificationName(
                Notification.Name("MeltypeUserDictionaryChanged"),object:nil,userInfo:nil,deliverImmediately:true)
        } catch {
            alert.messageText="ユーザー辞書を取り込めませんでした"
            alert.informativeText=error.localizedDescription
        }
        alert.runModal()
    }
    let root=FileManager.default.temporaryDirectory.appendingPathComponent("MeltypeDialog-"+UUID().uuidString,isDirectory:true)
    try FileManager.default.createDirectory(at:root,withIntermediateDirectories:true)
    defer {try? FileManager.default.removeItem(at:root)}
    let valid=root.appendingPathComponent("選択 辞書.plist")
    try PropertyListSerialization.data(fromPropertyList:[["reading":"カナ","phrase":"単語"]],format:.binary,options:0).write(to:valid)
    let malformed=root.appendingPathComponent("broken.plist")
    try Data("{ broken <plist".utf8).write(to:malformed)
    let unsupported=root.appendingPathComponent("empty.plist")
    try PropertyListSerialization.data(fromPropertyList:[],format:.xml,options:0).write(to:unsupported)
    let cases=["cancel","no source","no directory","success","duplicate","invalid plist","unsupported","missing source","invalid UTF8"]
    for name in cases {
        let oldFolder=root.appendingPathComponent("old "+name,isDirectory:true)
        let nativeFolder=root.appendingPathComponent("native "+name,isDirectory:true)
        if name == "duplicate" || name == "invalid UTF8" {
            for folder in [oldFolder,nativeFolder] {
                try FileManager.default.createDirectory(at:folder,withIntermediateDirectories:true)
                try (name == "duplicate" ? Data("かな\t単語\n".utf8) : Data([0xc0,0xaf])).write(to:folder.appendingPathComponent("userdict.txt"))
            }
        }
        response=name == "cancel" ? .cancel : .OK
        selected=name == "no source" ? nil : name == "invalid plist" ? malformed : name == "unsupported" ? unsupported : name == "missing source" ? root.appendingPathComponent("missing.plist") : valid
        trace=[];panelState=[];alertState=[];notifications=[]
        legacy {trace.append("directory");return name == "no directory" ? nil : oldFolder.path}
        let expectedTrace=trace,expectedPanel=panelState,expectedAlert=alertState,expectedNotifications=notifications
        trace=[];panelState=[];alertState=[];notifications=[]
        checkingNativeChooser = true
        let result=editor.showDictionaryImport {trace.append("directory");return name == "no directory" ? nil : nativeFolder.path}
        checkingNativeChooser = false
        precondition(trace==expectedTrace,"dialog order differs for \(name): \(trace), \(expectedTrace)")
        precondition(panelState==expectedPanel,"chooser defaults differ for \(name)")
        precondition(alertState==expectedAlert,"alert differs for \(name): \(alertState), \(expectedAlert)")
        precondition(notifications==expectedNotifications,"notification arguments differ for \(name)")
        precondition(result==(expectedTrace.contains("alert") ? 2 : 1))
        if name == "success" || name == "duplicate" {
            precondition(notifications==["MeltypeUserDictionaryChanged","true","true","true"])
            let expected=try Data(contentsOf:oldFolder.appendingPathComponent("userdict.txt"))
            let actual=try Data(contentsOf:nativeFolder.appendingPathComponent("userdict.txt"))
            precondition(actual==expected)
        }
        if name == "cancel" || name == "no source" || name == "no directory" {
            precondition(!FileManager.default.fileExists(atPath:nativeFolder.path),"cancelled dialog touched destination")
        }
    }
    // Use a real result alert as well: the file picker response and distributed
    // post remain intercepted, then press only this process's default button.
    restoreAlert()
    selected=valid;response = .OK
    var pressed=false
    var alertFinished=false
    let alertWatchdog=DispatchWorkItem {fatalError("owned dictionary result alert timed out")}
    DispatchQueue.global().asyncAfter(deadline:.now()+15,execute:alertWatchdog)
    defer {alertWatchdog.cancel()}
    func dismissOwnAlert() {
        guard !alertFinished else {return}
        if let window=NSApp.modalWindow,!(window is NSOpenPanel),
           let button=window.defaultButtonCell?.controlView as? NSButton {
            pressed=true
            button.performClick(nil)
        } else {DispatchQueue.main.asyncAfter(deadline:.now()+0.05,execute:dismissOwnAlert)}
    }
    DispatchQueue.main.asyncAfter(deadline:.now()+0.05,execute:dismissOwnAlert)
    let alertResult=editor.showDictionaryImport {root.appendingPathComponent("actual alert").path}
    alertFinished=true;alertWatchdog.cancel()
    precondition(alertResult==2 && pressed,"actual owned result alert was not acknowledged")
    print("macOS Rust dictionary dialog: actual owned NSAlert modal loop and default-button acknowledgement passed (chooser/notification intercepted)")
    let done=DispatchSemaphore(value:0)
    DispatchQueue.global().async {
        let result=editor.showDictionaryImport {fatalError("background dialog queried directory")}
        precondition(result==0)
        done.signal()
    }
    precondition(done.wait(timeout:.now()+5) == .success)
    print("macOS Rust dictionary dialog: 9 legacy comparisons with actual SDK panel/alert configuration, short-circuit order, zero/nonzero notifications, errors and private file bytes; background rejection passed (comparison modal responses/distributed post intercepted; real modal loops tested separately, OS delivery NOT_RUN)")
}

// A separate real chooser cancellation, after all scoped SDK intercepts restore.
func testDictionaryChooserCancel(_ editor:NativeEditorPolicy) {
    var cancelled=false
    var finished=false
    let watchdog=DispatchWorkItem {fatalError("owned dictionary chooser timed out")}
    DispatchQueue.global().asyncAfter(deadline:.now()+15,execute:watchdog)
    defer {watchdog.cancel()}
    func cancelOwnChooser() {
        guard !finished else {return}
        if let panel=NSApp.modalWindow as? NSOpenPanel {
            cancelled=true
            panel.cancel(nil)
        } else {DispatchQueue.main.asyncAfter(deadline:.now()+0.05,execute:cancelOwnChooser)}
    }
    DispatchQueue.main.asyncAfter(deadline:.now()+0.05,execute:cancelOwnChooser)
    let result=editor.showDictionaryImport {fatalError("real cancelled chooser queried data directory")}
    finished=true;watchdog.cancel()
    precondition(result==1 && cancelled)
    print("macOS Rust dictionary dialog: actual owned NSOpenPanel modal loop and cancellation passed (no file selected or modified)")
}
