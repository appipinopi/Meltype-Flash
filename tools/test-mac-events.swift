// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 0924haruto12

import Cocoa
import InputMethodKit
import Carbon.HIToolbox

// テストだけの利用状況スタブ。実際の実装は mac/Sources/MeltypeIME/Analytics.swift で、
// Aptabase を使うため境界試験ではリンクせず、送信もしない。
enum FlashAnalytics {
    static var isEnabled: Bool { false }
    static func setEnabled(_ enabled: Bool) {}
    static func start() {}
    static func track(_ event: String) {}
}

final class EventClient: NSObject, IMKTextInput {
 var document=""; var marked=""
 var lastMarkedValue:NSAttributedString?; var lastMarkedSelection:NSRange?
 func text(_ value:Any?)->String { (value as? NSAttributedString)?.string ?? (value as? String ?? "") }
 func insertText(_ string: Any!, replacementRange: NSRange) { let value=text(string); if replacementRange.location != NSNotFound { document=(document as NSString).replacingCharacters(in:replacementRange,with:value) } else {document+=value}; marked="" }
 func setMarkedText(_ string: Any!, selectionRange: NSRange, replacementRange: NSRange) {marked=text(string);lastMarkedValue=string as? NSAttributedString;lastMarkedSelection=selectionRange}
 func selectedRange()->NSRange {NSRange(location:(document as NSString).length,length:0)}
 func markedRange()->NSRange {marked.isEmpty ? NSRange(location:NSNotFound,length:0):NSRange(location:(document as NSString).length,length:(marked as NSString).length)}
 func attributedSubstring(from range:NSRange)->NSAttributedString! { let text=document as NSString; guard range.location != NSNotFound && range.location<=text.length else{return nil}; return NSAttributedString(string:text.substring(with:NSRange(location:range.location,length:min(range.length,text.length-range.location)))) }
 func length()->Int {(document as NSString).length}
 func characterIndex(for point:NSPoint, tracking mode:IMKLocationToOffsetMappingMode, inMarkedRange:UnsafeMutablePointer<ObjCBool>!)->Int {inMarkedRange?.pointee=false; return 0}
 func attributes(forCharacterIndex index:Int, lineHeightRectangle rectangle:UnsafeMutablePointer<NSRect>!)->[AnyHashable:Any]! {rectangle?.pointee=NSRect(x:0,y:0,width:1,height:20); return [:]}
 func validAttributesForMarkedText()->[Any]! {[]}
 func overrideKeyboard(withKeyboardNamed name:String!) {}
 func selectMode(_ identifier:String!) {}
 func supportsUnicode()->Bool {true}
 func bundleIdentifier()->String! {"local.meltype.event-client"}
 func windowLevel()->Int32 {0}
 func supportsProperty(_ property:TSMDocumentPropertyTag)->Bool {true}
 func uniqueClientIdentifierString()->String! {"synthetic-event-client"}
 func string(from range:NSRange, actualRange:UnsafeMutablePointer<NSRange>!)->String! {actualRange?.pointee=range;return attributedSubstring(from:range)?.string}
 func firstRect(forCharacterRange range:NSRange, actualRange:UnsafeMutablePointer<NSRange>!)->NSRect {actualRange?.pointee=range;return .zero}
}
#if !REAL_CONVERTER
final class MeltypeConverter {
 static let shared=MeltypeConverter()
 func clauses(for text:String,context:String?)->[(reading:String,text:String)] {[]}
 func candidates(for text:String)->[String] {[]}
}
#endif
var candidatesWindow: IMKCandidates? = nil
@main
struct EventTests {
 static func main() {
  if let directory=ProcessInfo.processInfo.environment["MELTYPE_RUST_DATA_DIR"] {
   precondition(NativeCore.shared.dataDirectory==directory,"Rust core ignored private data directory")
  }
  let app=NSApplication.shared
  app.setActivationPolicy(.prohibited)
  #if REAL_CONVERTER
  // Exercise the real product callbacks through Rust, not just direct converter calls.
  NativeCore.shared.initialize()
  #endif
  guard let server=IMKServer(name:"LocalMeltypeBoundaryTests",bundleIdentifier:Bundle.main.bundleIdentifier) else {fatalError("IMKServer creation failed")}
  var passed=0
  func check(_ name:String,_ body:(MeltypeInputController,EventClient)->Void) {
   let client=EventClient()
   // IMKInputController の初期化は OS のクライアントプロキシ専用なので nil で初期化する。
   // handle / commitComposition には合成 IMKTextInput クライアントを明示して渡す。
   guard let controller=MeltypeInputController(server:server,delegate:nil,client:nil) else {fatalError("Controller creation failed")}
   body(controller,client); passed+=1; print("PASS: \(name)")
  }
  func key(_ controller:MeltypeInputController,_ client:EventClient,_ chars:String,code:UInt16=0,flags:NSEvent.ModifierFlags=[])->Bool {
   let event=NSEvent.keyEvent(with:.keyDown,location:.zero,modifierFlags:flags,timestamp:0,windowNumber:0,context:nil,characters:chars,charactersIgnoringModifiers:chars,isARepeat:false,keyCode:code)!
   let consumed=controller.handle(event,client:client)
   if !consumed && !flags.contains(.command) && !flags.contains(.control) && chars.unicodeScalars.allSatisfy({$0.value>=0x20}) {
    // アプリが元イベントを受ける部分を 1 回だけ模擬する。
    client.insertText(chars,replacementRange:NSRange(location:NSNotFound,length:0))
   }
   return consumed
  }
  func type(_ controller:MeltypeInputController,_ client:EventClient,_ raw:String) {
   for scalar in raw.unicodeScalars { _=key(controller,client,String(scalar),code:UInt16(kVK_ANSI_A)) }
  }
  func equal(_ actual:String,_ expected:String) {
   precondition(Array(actual.unicodeScalars)==Array(expected.unicodeScalars),"expected \(expected.debugDescription), got \(actual.debugDescription)")
  }
  for (raw,event,expected) in [("e","\u{0301}\u{0300}","e\u{0301}\u{0300}"),("ka","\u{3099}\u{FE0F}","か\u{3099}\u{FE0F}"),("ka","\u{E0100}\u{E0101}","か\u{E0100}\u{E0101}"),("ka","😀","か😀"),("ka","👩‍👩‍👧‍👦","か👩‍👩‍👧‍👦"),("ka","e\u{0301}","かe\u{0301}")] {
   check("event \(event.debugDescription)") { controller,client in
    type(controller,client,raw)
    precondition(!key(controller,client,event),"external event must pass through")
    equal(client.document,expected); equal(client.marked,"")
   }
  }
  for action in ["Enter","Space","Tab","focus"] {
   check("protected \(action)") { controller,client in
    type(controller,client,"@kuraido")
    switch action {
    case "Enter": _=key(controller,client,"\r",code:UInt16(kVK_Return))
    case "Space": _=key(controller,client," ",code:UInt16(kVK_Space))
    case "Tab": _=key(controller,client,"\t",code:UInt16(kVK_Tab))
    default: controller.commitComposition(client)
    }
    equal(client.document,"@kuraido"+(action=="Space" ? " " : ""));equal(client.marked,"")
   }
  }
  for action in ["Enter","Space","Tab","focus"] {
   check("mixed protected \(action)") { controller,client in
    type(controller,client,"kyouha「@kuraido」ashita")
    switch action {
    case "Enter": _=key(controller,client,"\r",code:UInt16(kVK_Return))
    case "Space": _=key(controller,client," ",code:UInt16(kVK_Space));_=key(controller,client,"\r",code:UInt16(kVK_Return))
    case "Tab": _=key(controller,client,"\t",code:UInt16(kVK_Tab))
    default: controller.commitComposition(client)
    }
    #if REAL_CONVERTER
    precondition(client.document.contains("「@kuraido」"),"protected token changed")
    precondition(!client.document.contains("kyouha") && !client.document.contains("ashita"),"Japanese gaps skipped")
    #else
    equal(client.document,"きょうは「@kuraido」あした")
    #endif
    equal(client.marked,"")
   }
  }
  check("Backspace at protected boundary") { controller,client in
   type(controller,client,"./z]");_=key(controller,client,"",code:UInt16(kVK_Delete));equal(client.marked,"./z")
   _=key(controller,client,"\r",code:UInt16(kVK_Return));equal(client.document,"./z")
  }
  check("F6 override before marks") { controller,client in
   type(controller,client,"e");_=key(controller,client,"",code:UInt16(kVK_F6));_=key(controller,client,"\u{0301}\u{0300}");equal(client.document,"え\u{0301}\u{0300}")
  }
  check("F9 override") { controller,client in
   type(controller,client,"@kuraido");_=key(controller,client,"",code:UInt16(kVK_F9));_=key(controller,client,"\r",code:UInt16(kVK_Return));equal(client.document,"＠ｋｕｒａｉｄｏ")
  }
  check("JIS direct and kana modes") { controller,client in
   _=key(controller,client,"",code:UInt16(kVK_JIS_Eisu));type(controller,client,"ka");equal(client.document,"ka")
   _=key(controller,client,"",code:UInt16(kVK_JIS_Kana));type(controller,client,"ka");_=key(controller,client,"",code:UInt16(kVK_F6));_=key(controller,client,"\r",code:UInt16(kVK_Return));equal(client.document,"ka か")
  }
  check("Command shortcut commits once") { controller,client in
   type(controller,client,"@kuraido");precondition(!key(controller,client,"a",flags:.command));equal(client.document,"@kuraido");equal(client.marked,"")
  }
  #if MELTYPE_RUST_CORE
  check("Rust UTF16 marked composition and clear through actual IMK bridge") { controller,client in
   let view=CompositionView(text:"前😀後",converting:true,selectedIndex:0,selectedClause:1,hint:"",candidates:[],clauses:["前","😀","後"],meaning:nil,suggestion:nil)
   controller.apply(SessionResult(consumed:true,commits:[],view:view),to:client)
   equal(client.marked,"前😀後")
   precondition(client.lastMarkedValue?.length==4 && client.lastMarkedSelection==NSRange(location:4,length:0),"UTF16 marked payload/selection mismatch")
   controller.apply(SessionResult(consumed:true,commits:[],view:nil),to:client)
   equal(client.marked,"")
   precondition(client.lastMarkedValue?.length==0 && client.lastMarkedSelection==NSRange(location:0,length:0),"marked clear must retain attributed payload type")
  }
  check("Rust response loop dependent replacement") { controller,client in
   client.document="前😀"
   controller.apply(SessionResult(consumed:true,commits:[TextEdit(deleteBefore:3,text:"仮名",expect:"前😀"),TextEdit(deleteBefore:2,text:"終",expect:"仮名")],view:nil),to:client)
   equal(client.document,"終");equal(client.marked,"")
  }
  check("Rust response loop mismatch stops later edits and view") { controller,client in
   client.document="原文"
   let view=CompositionView(text:"表示禁止",converting:false,selectedIndex:0,selectedClause:0,hint:"",candidates:[],clauses:[],meaning:nil,suggestion:nil)
   controller.apply(SessionResult(consumed:true,commits:[TextEdit(deleteBefore:1,text:"変更禁止",expect:"違"),TextEdit(deleteBefore:0,text:"後続禁止",expect:nil)],view:view),to:client)
   equal(client.document,"原文");equal(client.marked,"")
  }
  #endif
  #if REAL_CONVERTER
  let converter=MeltypeConverter.shared
  let expectedConversion=converter.clauses(for:"きょう",context:nil).map(\.text).joined()
  precondition(!expectedConversion.isEmpty && expectedConversion != "きょう", "real converter did not produce kanji")
  check("actual converter through Rust/IMK Space and commit") { controller,client in
   type(controller,client,"kyou")
   _=key(controller,client," ",code:UInt16(kVK_Space))
   _=key(controller,client,"\r",code:UInt16(kVK_Return))
   equal(client.document,expectedConversion)
   equal(client.marked,"")
  }
  let expectedTabConversion=converter.clauses(for:"てんき",context:nil).map(\.text).joined()
  precondition(!expectedTabConversion.isEmpty && expectedTabConversion != "てんき", "Tab fixture requires a real kanji result")
  check("actual converter through Rust/IMK Tab with predictions disabled") { controller,client in
   type(controller,client,"tenki")
   _=key(controller,client,"\t",code:UInt16(kVK_Tab))
   _=key(controller,client,"\r",code:UInt16(kVK_Return))
   equal(client.document,expectedTabConversion)
   equal(client.marked,"")
  }
  let candidates=converter.candidates(for:"きょう")
  precondition(candidates.contains("今日"),"real dictionary conversion missing: \(candidates)")
  let clauses=converter.clauses(for:"きょうはいいてんき",context:nil)
  precondition(clauses.map(\.reading).joined()=="きょうはいいてんき","real clause readings lost")
  precondition(clauses.map(\.text).joined().contains("今日"),"real clause conversion missing")
  guard let directory=NativeCore.shared.dataDirectory else {fatalError("missing data directory")}
  precondition(FileManager.default.fileExists(atPath:directory+"/azooKey"),"converter ignored isolated data directory")
  passed+=3
  print("PASS: actual azooKey candidates, clause reading coverage, isolated data directory")
  precondition(NativeCore.shared.spellCheckCount > 0,"Rust did not invoke the registered NSSpellChecker callback")
  passed+=1
  print("PASS: registered NSSpellChecker through Rust (\(NativeCore.shared.spellCheckCount) actual checks)")
  #endif
  print("\(passed) Mac event/integration cases passed (synthetic client; OS IME registration and app GUI NOT_RUN)")
 }
}
