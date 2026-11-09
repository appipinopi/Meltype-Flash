// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
import Foundation

struct NativeEditorPolicy {
    private typealias ContextFunction = @convention(c) (UInt64, UInt64, Int32, UnsafeMutablePointer<UInt64>?) -> Int32
    private typealias ReplacementFunction = @convention(c) (UInt64, UInt64, UInt64, Int64, UnsafePointer<UInt16>?, UInt32, UnsafeMutablePointer<UInt64>?) -> Int32
    private typealias EqualFunction = @convention(c) (UnsafePointer<UInt16>?, UInt32, UnsafePointer<UInt16>?, UInt32) -> Int32
    private typealias ApplyFunction = @convention(c) (UnsafePointer<UInt8>?, UInt32, UnsafeMutableRawPointer?, NativeEditHost) -> Int32
    private typealias CompositionFunction = @convention(c) (UnsafePointer<UInt8>?, UInt32, UnsafeMutableRawPointer?, NativeCompositionHost) -> Int32
    private typealias CandidatesFunction = @convention(c) (UnsafePointer<UInt8>?, UInt32, UnsafeMutableRawPointer?, NativeCandidatesHost) -> Int32
    private typealias MeaningUpdate = @convention(c) (UnsafeMutableRawPointer?, Int64, UnsafePointer<UInt16>?, UInt32, Int32) -> Int32
    private typealias MeaningMatches = @convention(c) (UnsafeMutableRawPointer?, Int64, UnsafePointer<UInt16>?, UInt32) -> Int32
    private typealias MeaningDelay = @convention(c) () -> UInt32
    private typealias MeaningSchedule = @convention(c) (UnsafeMutableRawPointer?, NativeMeaningCallback) -> Int32
    private typealias WindowFunction = @convention(c) (UnsafeMutableRawPointer?, UInt32, Int64, UnsafePointer<UInt16>?, UInt32) -> Int32
    private typealias ServerCreate = @convention(c) (UnsafePointer<UInt16>?, UInt32, UnsafePointer<UInt16>?, UInt32) -> UnsafeMutableRawPointer?
    private typealias CandidatesCreate = @convention(c) (UnsafeMutableRawPointer?, UInt64) -> UnsafeMutableRawPointer?
    private typealias ApplicationRun = @convention(c) () -> Int32
    private typealias ServicesRegister = @convention(c) (UnsafeMutableRawPointer?, UnsafePointer<UInt16>?, UInt32) -> Int32
    private typealias ModeServicesCreate = @convention(c) (UnsafeMutableRawPointer?, NativeModeServiceHost) -> UnsafeMutableRawPointer?
    private typealias ModeServicesTarget = @convention(c) (UnsafeMutableRawPointer?, UnsafeMutableRawPointer?, Int32) -> Int32
    private typealias ModeServicesDispatch = @convention(c) (UnsafeMutableRawPointer?, UnsafePointer<UInt16>?, UInt32, Int32) -> Int32
    private typealias ModeServicesError = @convention(c) (Int32) -> UnsafePointer<CChar>?
    private typealias SourceMatches = @convention(c) (UnsafePointer<UInt16>?, UInt32) -> Int32
    private let modeServicesCreate: ModeServicesCreate
    private let modeServicesTargetFunction: ModeServicesTarget
    private let modeServicesDispatch: ModeServicesDispatch
    private let modeServicesErrorFunction: ModeServicesError
    private let sourceMatches: SourceMatches
    private typealias Suggestion = @convention(c) (UnsafeMutableRawPointer?, UnsafePointer<UInt16>?, UInt32, Int32, UnsafePointer<Double>?, Int32) -> UnsafeMutableRawPointer?
    private typealias IndicatorContent = @convention(c) (UInt32,Int32) -> UnsafeMutableRawPointer?
    private typealias IndicatorFrame = @convention(c) (UInt32,UnsafePointer<Double>?,UnsafePointer<Double>?,UnsafeMutablePointer<Double>?) -> Int32
    private typealias IndicatorCreate = @convention(c) (NativeIndicatorEnabled) -> UnsafeMutableRawPointer?
    private typealias IndicatorUpdate = @convention(c) (UnsafeMutableRawPointer?,UInt32,UInt32,UnsafePointer<Double>?) -> Int32
    private typealias IndicatorPanel = @convention(c) (UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer?
    private let indicatorCreateFunction: IndicatorCreate
    private let indicatorUpdateFunction: IndicatorUpdate
    private let indicatorPanelFunction: IndicatorPanel
    private let indicatorContentFunction: IndicatorContent
    private let indicatorFrameFunction: IndicatorFrame
    private let suggestionFunction: Suggestion
    private typealias ModeAfter = @convention(c) (UInt32,UInt32) -> Int32
    private typealias IndicatorPreference = @convention(c) (UnsafeMutableRawPointer?,UInt32,Int32) -> Int32
    private typealias SelectionCreate = @convention(c) (UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer?
    private typealias SelectionUpdate = @convention(c) (UnsafeMutableRawPointer?,UInt32,UInt32) -> Int32
    private let modeAfterFunction: ModeAfter
    private let indicatorPreferenceFunction: IndicatorPreference
    private let selectionCreateFunction: SelectionCreate
    private let selectionUpdateFunction: SelectionUpdate
    private typealias MenuCreate = @convention(c) (UnsafeMutableRawPointer?,Int32,Int32,Int32) -> UnsafeMutableRawPointer?
    private let menuCreateFunction: MenuCreate
    private typealias DictionaryEntries = @convention(c) (UnsafePointer<UInt8>?,UInt32,UnsafeMutablePointer<UnsafeMutableRawPointer?>?) -> UnsafeMutablePointer<CChar>?
    private typealias DictionaryMerge = @convention(c) (UnsafePointer<UInt8>?,UInt32) -> UnsafeMutablePointer<CChar>?
    private typealias FreeDictionaryJSON = @convention(c) (UnsafeMutablePointer<CChar>?) -> Void
    private typealias DictionaryImport = @convention(c) (UnsafeMutableRawPointer?,UnsafePointer<UInt16>?,UInt32,UnsafeMutablePointer<UnsafeMutableRawPointer?>?) -> Int64
    private typealias DictionaryDialog = @convention(c) (UnsafeMutableRawPointer?, NativeDictionaryDirectory) -> Int32
    private typealias InputSourcesObject = @convention(c) (UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer?
    private typealias InputSourcesRepair = @convention(c) (UnsafeMutableRawPointer?) -> Int32
    private typealias InputSourceRegister = @convention(c) () -> Int32
    private let inputSourcesNormalized: InputSourcesObject
    private let inputSourcesRepair: InputSourcesRepair
    private let inputSourceRegister: InputSourceRegister
    private let dictionaryDialogFunction: DictionaryDialog
    private let dictionaryImportFunction: DictionaryImport
    private let dictionaryEntriesFunction: DictionaryEntries
    private let dictionaryMergeFunction: DictionaryMerge
    private let dictionaryFreeFunction: FreeDictionaryJSON
    private let applicationRun: ApplicationRun
    private let servicesRegister: ServicesRegister
    private let serverCreate: ServerCreate
    private let candidatesCreate: CandidatesCreate
    private let windowFunction: WindowFunction
    private let meaningSchedule: MeaningSchedule
    private let meaningDelay: MeaningDelay
    var meaningDelayMilliseconds: Int { Int(meaningDelay()) }
    private let meaningUpdate: MeaningUpdate
    private let meaningMatches: MeaningMatches
    private let candidatesFunction: CandidatesFunction
    private let compositionFunction: CompositionFunction
    private let applyFunction: ApplyFunction
    private let context: ContextFunction
    private let replacement: ReplacementFunction
    private let equal: EqualFunction
    init?(lookup: (String) -> UnsafeMutableRawPointer?) {
        guard let context = lookup("meltype_macos_context_range"),
              let replacement = lookup("meltype_macos_replacement_range"),
              let equal = lookup("meltype_macos_expected_equal"),
              let apply = lookup("meltype_macos_apply_utf8"),
              let composition = lookup("meltype_macos_composition_utf8"),
              let candidates = lookup("meltype_macos_candidates_utf8"),
              let meaningUpdate = lookup("meltype_macos_meaning_update"),
              let meaningMatches = lookup("meltype_macos_meaning_matches"),
              let meaningDelay = lookup("meltype_macos_meaning_delay_ms"),
              let meaningSchedule = lookup("meltype_macos_meaning_schedule"),
              let windowFunction = lookup("meltype_macos_candidate_window"),
              let serverCreate = lookup("meltype_macos_server_create"),
              let candidatesCreate = lookup("meltype_macos_candidates_create"),
              let applicationRun = lookup("meltype_macos_application_run"),
              let servicesRegister = lookup("meltype_macos_services_register"),
              let modeServicesCreate=lookup("meltype_macos_mode_services_create"),
              let modeServicesTarget=lookup("meltype_macos_mode_services_target"),
              let modeServicesDispatch=lookup("meltype_macos_mode_services_dispatch"),
              let modeServicesError=lookup("meltype_macos_services_error"),
              let sourceMatches=lookup("meltype_macos_input_source_matches"),
              let suggestion=lookup("meltype_macos_suggestion"),
              let indicatorContent=lookup("meltype_macos_indicator_content"),
              let indicatorFrame=lookup("meltype_macos_indicator_frame"),
              let indicatorCreate=lookup("meltype_macos_indicator_create"),
              let indicatorUpdate=lookup("meltype_macos_indicator_update"),
              let indicatorPanel=lookup("meltype_macos_indicator_panel"),
              let modeAfter=lookup("meltype_macos_mode_after"),
              let indicatorPreference=lookup("meltype_macos_indicator_preference"),
              let selectionCreate=lookup("meltype_macos_mode_selection_create"),
              let selectionUpdate=lookup("meltype_macos_mode_selection"),
              let menuCreate=lookup("meltype_macos_menu"),
              let dictionaryEntries=lookup("meltype_macos_dictionary_entries"),
              let dictionaryMerge=lookup("meltype_macos_dictionary_merge"),
              let dictionaryFree=lookup("meltype_free"),
              let dictionaryImport=lookup("meltype_macos_dictionary_import"),
              let dictionaryDialog=lookup("meltype_macos_dictionary_dialog"),
              let inputSourcesNormalized=lookup("meltype_macos_input_sources_normalized"),
              let inputSourcesRepair=lookup("meltype_macos_input_sources_repair"),
              let inputSourceRegister=lookup("meltype_macos_input_source_register") else { return nil }
        self.modeServicesCreate=unsafeBitCast(modeServicesCreate,to:ModeServicesCreate.self)
        self.modeServicesTargetFunction=unsafeBitCast(modeServicesTarget,to:ModeServicesTarget.self)
        self.modeServicesDispatch=unsafeBitCast(modeServicesDispatch,to:ModeServicesDispatch.self)
        self.modeServicesErrorFunction=unsafeBitCast(modeServicesError,to:ModeServicesError.self)
        self.sourceMatches=unsafeBitCast(sourceMatches,to:SourceMatches.self)
        self.indicatorCreateFunction=unsafeBitCast(indicatorCreate,to:IndicatorCreate.self)
        self.indicatorUpdateFunction=unsafeBitCast(indicatorUpdate,to:IndicatorUpdate.self)
        self.indicatorPanelFunction=unsafeBitCast(indicatorPanel,to:IndicatorPanel.self)
        self.indicatorContentFunction=unsafeBitCast(indicatorContent,to:IndicatorContent.self)
        self.indicatorFrameFunction=unsafeBitCast(indicatorFrame,to:IndicatorFrame.self)
        self.suggestionFunction=unsafeBitCast(suggestion,to:Suggestion.self)
        self.modeAfterFunction=unsafeBitCast(modeAfter,to:ModeAfter.self)
        self.indicatorPreferenceFunction=unsafeBitCast(indicatorPreference,to:IndicatorPreference.self)
        self.selectionCreateFunction=unsafeBitCast(selectionCreate,to:SelectionCreate.self)
        self.selectionUpdateFunction=unsafeBitCast(selectionUpdate,to:SelectionUpdate.self)
        self.menuCreateFunction=unsafeBitCast(menuCreate,to:MenuCreate.self)
        self.inputSourcesNormalized=unsafeBitCast(inputSourcesNormalized,to:InputSourcesObject.self)
        self.inputSourcesRepair=unsafeBitCast(inputSourcesRepair,to:InputSourcesRepair.self)
        self.inputSourceRegister=unsafeBitCast(inputSourceRegister,to:InputSourceRegister.self)
        self.dictionaryDialogFunction=unsafeBitCast(dictionaryDialog,to:DictionaryDialog.self)
        self.dictionaryImportFunction=unsafeBitCast(dictionaryImport,to:DictionaryImport.self)
        self.dictionaryEntriesFunction=unsafeBitCast(dictionaryEntries,to:DictionaryEntries.self)
        self.dictionaryMergeFunction=unsafeBitCast(dictionaryMerge,to:DictionaryMerge.self)
        self.dictionaryFreeFunction=unsafeBitCast(dictionaryFree,to:FreeDictionaryJSON.self)
        self.applicationRun = unsafeBitCast(applicationRun, to: ApplicationRun.self)
        self.servicesRegister = unsafeBitCast(servicesRegister, to: ServicesRegister.self)
        self.serverCreate = unsafeBitCast(serverCreate, to: ServerCreate.self)
        self.candidatesCreate = unsafeBitCast(candidatesCreate, to: CandidatesCreate.self)
        self.windowFunction = unsafeBitCast(windowFunction, to: WindowFunction.self)
        self.meaningSchedule = unsafeBitCast(meaningSchedule, to: MeaningSchedule.self)
        self.meaningDelay = unsafeBitCast(meaningDelay, to: MeaningDelay.self)
        self.meaningUpdate = unsafeBitCast(meaningUpdate, to: MeaningUpdate.self)
        self.meaningMatches = unsafeBitCast(meaningMatches, to: MeaningMatches.self)
        self.candidatesFunction = unsafeBitCast(candidates, to: CandidatesFunction.self)
        self.compositionFunction = unsafeBitCast(composition, to: CompositionFunction.self)
        self.applyFunction = unsafeBitCast(apply, to: ApplyFunction.self)
        self.context = unsafeBitCast(context, to: ContextFunction.self)
        self.replacement = unsafeBitCast(replacement, to: ReplacementFunction.self)
        self.equal = unsafeBitCast(equal, to: EqualFunction.self)
    }
    @discardableResult
    func apply(_ result: SessionResult, marked: @escaping () -> NSRange, selected: @escaping () -> NSRange,
               substring: @escaping (NSRange) -> String?, insert: @escaping (String, NSRange) -> Void,
               clearMarked: @escaping () -> Void, render: @escaping () -> Void) -> Bool {
        guard let data = try? JSONEncoder().encode(result), let count = UInt32(exactly: data.count) else { return false }
        let sink = NativeEditSink(marked: marked, selected: selected, substring: substring, insert: insert, clearMarked: clearMarked, render: render)
        return withExtendedLifetime(sink) {
            data.withUnsafeBytes { bytes in
                applyFunction(bytes.bindMemory(to: UInt8.self).baseAddress, count, Unmanaged.passUnretained(sink).toOpaque(), nativeEditHost) != 0
            }
        }
    }
    @discardableResult
    func composition(_ view: CompositionView, mark: @escaping (Int, NSRange) -> Void, publish: @escaping (Int) -> Void) -> Bool {
        guard let data = try? JSONEncoder().encode(view), let count = UInt32(exactly: data.count) else { return false }
        let sink = NativeCompositionSink(mark: mark, publish: publish)
        return withExtendedLifetime(sink) {
            data.withUnsafeBytes { bytes in compositionFunction(bytes.bindMemory(to: UInt8.self).baseAddress, count, Unmanaged.passUnretained(sink).toOpaque(), nativeCompositionHost) != 0 }
        }
    }
    @discardableResult
    func candidates(_ view: CompositionView, previous: [String], visible: Bool, windowIndex: Int,
                    operation: @escaping (UInt32, Int) -> Int32) -> Bool {
        struct Update: Encodable { let candidates: [String]; let selectedIndex: Int; let previous: [String]; let visible: Bool; let windowIndex: Int }
        let update = Update(candidates: view.candidates, selectedIndex: view.selectedIndex, previous: previous, visible: visible, windowIndex: windowIndex)
        guard let data = try? JSONEncoder().encode(update), let count = UInt32(exactly: data.count) else { return false }
        let sink = NativeCandidatesSink(operation)
        return withExtendedLifetime(sink) {
            data.withUnsafeBytes { candidatesFunction($0.bindMemory(to: UInt8.self).baseAddress, count, Unmanaged.passUnretained(sink).toOpaque(), nativeCandidatesHost) != 0 }
        }
    }
    func createModeServices(owner: AnyObject, host: NativeModeServiceHost) -> AnyObject? {
        let pointer=withExtendedLifetime(owner) { modeServicesCreate(Unmanaged.passUnretained(owner).toOpaque(),host) }
        guard let pointer else { return nil }
        return Unmanaged<AnyObject>.fromOpaque(pointer).takeRetainedValue()
    }
    func modeServicesTarget(_ provider: AnyObject, target: AnyObject, active: Bool) {
        withExtendedLifetime((provider,target)) {
            _=modeServicesTargetFunction(Unmanaged.passUnretained(provider).toOpaque(),Unmanaged.passUnretained(target).toOpaque(),active ? 1 : 0)
        }
    }
    func dispatchModeServices(_ provider: AnyObject?, identifier: String?) -> Int32 {
        let units=Array((identifier ?? "").utf16)
        guard let count=UInt32(exactly:units.count) else { return 2 }
        return withExtendedLifetime(provider) {
            units.withUnsafeBufferPointer { modeServicesDispatch(provider.map { Unmanaged.passUnretained($0).toOpaque() },$0.baseAddress,count,identifier == nil ? 0 : 1) }
        }
    }
    func modeServicesError(_ code: Int32) -> NSString? {
        guard code != 0, let pointer=modeServicesErrorFunction(code) else { return nil }
        return String(cString:pointer) as NSString
    }
    func inputSourceMatches(expected: String) -> Bool {
        let units=Array(expected.utf16)
        guard let count=UInt32(exactly:units.count) else { return false }
        return units.withUnsafeBufferPointer { sourceMatches($0.baseAddress,count) != 0 }
    }
    func suggestion(panel: AnyObject?, text: String?, caret: NSRect, selected: Bool) -> AnyObject? {
        let units=Array((text ?? "").utf16), rectangle=[Double(caret.origin.x),Double(caret.origin.y),Double(caret.size.width),Double(caret.size.height)]
        guard let count=UInt32(exactly:units.count) else { return nil }
        let pointer=withExtendedLifetime(panel) {
            units.withUnsafeBufferPointer { textBuffer in
                rectangle.withUnsafeBufferPointer { bounds in
                    suggestionFunction(panel.map { Unmanaged.passUnretained($0).toOpaque() },textBuffer.baseAddress,count,text == nil ? 0 : 1,bounds.baseAddress,selected ? 1 : 0)
                }
            }
        }
        guard let pointer else { return nil }
        return Unmanaged<AnyObject>.fromOpaque(pointer).takeRetainedValue()
    }
    func createIndicator(enabled: NativeIndicatorEnabled) -> AnyObject? {
        guard let pointer=indicatorCreateFunction(enabled) else {return nil}
        return Unmanaged<AnyObject>.fromOpaque(pointer).takeRetainedValue()
    }
    @discardableResult
    func updateIndicator(_ owner: AnyObject,operation:UInt32,mode:UInt32=0,caret:NSRect?=nil)->Bool {
        let values: [Double] = caret.map { rect -> [Double] in [Double(rect.origin.x),Double(rect.origin.y),Double(rect.size.width),Double(rect.size.height)] } ?? [0.0,0.0,0.0,0.0]
        return withExtendedLifetime(owner) {values.withUnsafeBufferPointer {
            indicatorUpdateFunction(Unmanaged.passUnretained(owner).toOpaque(),operation,mode,caret == nil ? nil : $0.baseAddress) != 0
        }}
    }
    func indicatorPanel(_ owner:AnyObject)->AnyObject? {
        guard let pointer=withExtendedLifetime(owner,{indicatorPanelFunction(Unmanaged.passUnretained(owner).toOpaque())}) else {return nil}
        return Unmanaged<AnyObject>.fromOpaque(pointer).takeRetainedValue()
    }
    func indicatorContent(mode: UInt32,compact: Bool) -> AnyObject? {
        guard let pointer=indicatorContentFunction(mode,compact ? 1 : 0) else { return nil }
        return Unmanaged<AnyObject>.fromOpaque(pointer).takeRetainedValue()
    }
    func indicatorFrame(operation: UInt32,caret: NSRect?,visible: NSRect) -> NSRect? {
        func values(_ rect:NSRect)->[Double] {[Double(rect.origin.x),Double(rect.origin.y),Double(rect.size.width),Double(rect.size.height)]}
        let caretValues=values(caret ?? .zero),visibleValues=values(visible)
        var output=[Double](repeating:0,count:4)
        let result=caretValues.withUnsafeBufferPointer { c in visibleValues.withUnsafeBufferPointer { v in
            indicatorFrameFunction(operation,caret == nil ? nil : c.baseAddress,v.baseAddress,&output)
        } }
        guard result != 0 else { return nil }
        return NSRect(x:output[0],y:output[1],width:output[2],height:output[3])
    }
    func modeAfter(action:UInt32,current:UInt32)->Int32 {modeAfterFunction(action,current)}
    func indicatorPreference(_ defaults:UserDefaults,value:Bool?=nil)->Bool {
        withExtendedLifetime(defaults) {indicatorPreferenceFunction(Unmanaged.passUnretained(defaults).toOpaque(),value == nil ? 0 : 1,value == true ? 1 : 0)>0}
    }
    func createModeSelection(_ defaults:UserDefaults)->AnyObject? {
        guard let pointer=withExtendedLifetime(defaults,{selectionCreateFunction(Unmanaged.passUnretained(defaults).toOpaque())}) else{return nil}
        return Unmanaged<AnyObject>.fromOpaque(pointer).takeRetainedValue()
    }
    func modeSelection(_ owner:AnyObject,mode:UInt32?=nil)->Int32 {
        withExtendedLifetime(owner) {selectionUpdateFunction(Unmanaged.passUnretained(owner).toOpaque(),mode == nil ? 0 : 1,mode ?? 0)}
    }
    func inputMenu(target:AnyObject,direct:Bool,code:Bool,indicator:Bool)->AnyObject? {
        guard let pointer=withExtendedLifetime(target,{menuCreateFunction(Unmanaged.passUnretained(target).toOpaque(),direct ? 1 : 0,code ? 1 : 0,indicator ? 1 : 0)}) else {return nil}
        return Unmanaged<AnyObject>.fromOpaque(pointer).takeRetainedValue()
    }
    func normalizedInputSources(_ sources:[[String:Any]])->[[String:Any]] {
        let array=sources as NSArray
        guard let pointer=withExtendedLifetime(array,{inputSourcesNormalized(Unmanaged.passUnretained(array).toOpaque())}) else {preconditionFailure("Rust input-source normalization failed")}
        let result=Unmanaged<AnyObject>.fromOpaque(pointer).takeRetainedValue()
        guard let rows=result as? [[String:Any]] else {preconditionFailure("Rust input-source result invalid")}
        return rows
    }
    @discardableResult
    func repairInputSources(_ defaults:UserDefaults)->Bool {
        withExtendedLifetime(defaults) {inputSourcesRepair(Unmanaged.passUnretained(defaults).toOpaque()) != 0}
    }
    func registerInputSource()->Int32 {inputSourceRegister()}
    @discardableResult
    func showDictionaryImport(directory:@escaping () -> String?) -> Int32 {
        let sink=NativeDictionaryDirectorySink(directory)
        return withExtendedLifetime(sink) {
            dictionaryDialogFunction(Unmanaged.passUnretained(sink).toOpaque(),nativeDictionaryDirectory)
        }
    }
    func importDictionary(_ source:URL,directory:String) throws -> Int {
        let units=Array(directory.utf16)
        guard let count=UInt32(exactly:units.count) else{throw MacUserDictionary.ImportError.unsupported}
        let source=source as NSURL
        var failure:UnsafeMutableRawPointer?
        let result=withExtendedLifetime(source) {units.withUnsafeBufferPointer {
            dictionaryImportFunction(Unmanaged.passUnretained(source).toOpaque(),$0.baseAddress,count,&failure)
        }}
        guard result>=0 else {
            if let failure {throw Unmanaged<NSError>.fromOpaque(failure).takeRetainedValue()}
            throw MacUserDictionary.ImportError.unsupported
        }
        return Int(result)
    }
    func dictionaryEntries(_ data:Data) throws -> [(String,String)] {
        guard let count=UInt32(exactly:data.count) else {throw MacUserDictionary.ImportError.unsupported}
        var failure:UnsafeMutableRawPointer?
        let pointer=data.withUnsafeBytes {dictionaryEntriesFunction($0.bindMemory(to:UInt8.self).baseAddress,count,&failure)}
        guard let pointer else {
            if let failure {throw Unmanaged<NSError>.fromOpaque(failure).takeRetainedValue()}
            throw MacUserDictionary.ImportError.unsupported
        }
        defer {dictionaryFreeFunction(pointer)}
        let rows=try JSONDecoder().decode([[String]].self,from:Data(String(cString:pointer).utf8))
        guard !rows.isEmpty else {throw MacUserDictionary.ImportError.unsupported}
        return rows.map {($0[0],$0[1])}
    }
    func dictionaryMerge(_ entries:[(String,String)],existing:String)->(String,Int) {
        struct Request:Encodable {let entries:[[String]];let existing:String}
        struct Reply:Decodable {let text:String;let count:Int}
        guard let data=try? JSONEncoder().encode(Request(entries:entries.map {[$0.0,$0.1]},existing:existing)),let count=UInt32(exactly:data.count),
              let pointer=data.withUnsafeBytes({dictionaryMergeFunction($0.bindMemory(to:UInt8.self).baseAddress,count)}) else {preconditionFailure("Rust dictionary merge unavailable")}
        defer {dictionaryFreeFunction(pointer)}
        guard let reply=try? JSONDecoder().decode(Reply.self,from:Data(String(cString:pointer).utf8)) else {preconditionFailure("Rust dictionary merge invalid result")}
        return (reply.text,reply.count)
    }
    func runApplication() -> Bool { applicationRun() != 0 }
    func registerServices(provider: AnyObject, portName: String) -> Bool {
        let units=Array(portName.utf16)
        guard let count=UInt32(exactly:units.count) else { return false }
        return withExtendedLifetime(provider) {
            units.withUnsafeBufferPointer { servicesRegister(Unmanaged.passUnretained(provider).toOpaque(),$0.baseAddress,count) != 0 }
        }
    }
    func createServer(name: String, bundleIdentifier: String) -> AnyObject? {
        let nameUnits=Array(name.utf16), bundleUnits=Array(bundleIdentifier.utf16)
        guard let nameCount=UInt32(exactly:nameUnits.count), let bundleCount=UInt32(exactly:bundleUnits.count) else { return nil }
        let pointer=nameUnits.withUnsafeBufferPointer { name in
            bundleUnits.withUnsafeBufferPointer { bundle in serverCreate(name.baseAddress,nameCount,bundle.baseAddress,bundleCount) }
        }
        guard let pointer else { return nil }
        return Unmanaged<AnyObject>.fromOpaque(pointer).takeRetainedValue()
    }
    func createCandidates(server: AnyObject, panelType: UInt64) -> AnyObject? {
        let pointer=withExtendedLifetime(server) { candidatesCreate(Unmanaged.passUnretained(server).toOpaque(),panelType) }
        guard let pointer else { return nil }
        return Unmanaged<AnyObject>.fromOpaque(pointer).takeRetainedValue()
    }
    func candidateWindow(_ window: AnyObject, operation: UInt32, argument: Int = 0, text: String? = nil) -> Int32 {
        let units = Array((text ?? "").utf16)
        guard let count = UInt32(exactly: units.count) else { return 0 }
        return withExtendedLifetime(window) {
            units.withUnsafeBufferPointer { windowFunction(Unmanaged.passUnretained(window).toOpaque(), operation, Int64(argument), $0.baseAddress, count) }
        }
    }
    @discardableResult
    func scheduleMeaning(_ action: @escaping () -> Void) -> Bool {
        let context = Unmanaged.passRetained(NativeMeaningSink(action)).toOpaque()
        if meaningSchedule(context, nativeMeaningCallback) != 0 { return true }
        Unmanaged<NativeMeaningSink>.fromOpaque(context).release()
        return false
    }
    func reserveMeaning(_ handle: UnsafeMutableRawPointer?, index: Int, text: String?) -> Bool {
        let units = Array((text ?? "").utf16)
        guard let count = UInt32(exactly: units.count) else { return false }
        return units.withUnsafeBufferPointer { meaningUpdate(handle, Int64(index), $0.baseAddress, count, text == nil ? 0 : 1) != 0 }
    }
    func matchesMeaning(_ handle: UnsafeMutableRawPointer?, index: Int, text: String) -> Bool {
        let units = Array(text.utf16)
        guard let count = UInt32(exactly: units.count) else { return false }
        return units.withUnsafeBufferPointer { meaningMatches(handle, Int64(index), $0.baseAddress, count) != 0 }
    }
    func contextRanges(selection: NSRange, code: Bool) -> (NSRange, NSRange)? {
        guard let location = UInt64(exactly: selection.location), let length = UInt64(exactly: selection.length) else { return nil }
        var output = [UInt64](repeating: 0, count: 4)
        guard context(location, length, code ? 1 : 0, &output) != 0 else { return nil }
        return (NSRange(location: Int(output[0]), length: Int(output[1])), NSRange(location: Int(output[2]), length: Int(output[3])))
    }
    func replacementRange(marked: NSRange, selected: NSRange, remove: Int, expected: String?) -> NSRange? {
        guard let markedLocation = UInt64(exactly: marked.location), let markedLength = UInt64(exactly: marked.length),
              let selectedLocation = UInt64(exactly: selected.location), remove <= 0 || expected != nil else { return nil }
        let units = Array((expected ?? "").utf16)
        guard let length = UInt32(exactly: units.count) else { return nil }
        var output = [UInt64](repeating: 0, count: 2)
        let success = units.withUnsafeBufferPointer { replacement(markedLocation, markedLength, selectedLocation, Int64(remove), $0.baseAddress, length, &output) }
        guard success != 0 else { return nil }
        return NSRange(location: Int(output[0]), length: Int(output[1]))
    }
    func expectedEqual(_ actual: String, _ expected: String) -> Bool {
        let left = Array(actual.utf16), right = Array(expected.utf16)
        guard let leftCount = UInt32(exactly: left.count), let rightCount = UInt32(exactly: right.count) else { return false }
        return left.withUnsafeBufferPointer { a in right.withUnsafeBufferPointer { b in
            equal(a.baseAddress, leftCount, b.baseAddress, rightCount) != 0
        } }
    }
}

private typealias NativeEditHost = @convention(c) (UnsafeMutableRawPointer?, UInt32, UInt64, UInt64, UnsafePointer<UInt16>?, UInt32, UnsafeMutablePointer<UInt64>?) -> Int32
private final class NativeEditSink {
    let marked: () -> NSRange
    let selected: () -> NSRange
    let substring: (NSRange) -> String?
    let insert: (String, NSRange) -> Void
    let clearMarked: () -> Void
    let render: () -> Void
    var buffer: UnsafeMutablePointer<UInt16>?
    init(marked: @escaping () -> NSRange, selected: @escaping () -> NSRange, substring: @escaping (NSRange) -> String?, insert: @escaping (String, NSRange) -> Void, clearMarked: @escaping () -> Void, render: @escaping () -> Void) {
        self.marked = marked; self.selected = selected; self.substring = substring; self.insert = insert; self.clearMarked = clearMarked; self.render = render
    }
    deinit { buffer?.deallocate() }
}
private let nativeEditHost: NativeEditHost = { context, operation, location, length, text, count, output in
    guard let context else { return 0 }
    let sink = Unmanaged<NativeEditSink>.fromOpaque(context).takeUnretainedValue()
    switch operation {
    case 0, 1:
        guard let output else { return 0 }
        let range = operation == 0 ? sink.marked() : sink.selected()
        guard let at = UInt64(exactly: range.location), let size = UInt64(exactly: range.length) else { return 0 }
        output[0] = at; output[1] = size
    case 2:
        guard let output, let at = Int(exactly: location), let size = Int(exactly: length),
              let value = sink.substring(NSRange(location: at, length: size)) else { return 0 }
        let units = Array(value.utf16)
        guard UInt32(exactly: units.count) != nil else { return 0 }
        sink.buffer?.deallocate()
        let buffer = UnsafeMutablePointer<UInt16>.allocate(capacity: max(1, units.count))
        for (index, unit) in units.enumerated() { buffer[index] = unit }
        sink.buffer = buffer
        output[0] = UInt64(UInt(bitPattern: buffer)); output[1] = UInt64(units.count)
    case 3:
        guard let at = Int(exactly: location), let size = Int(exactly: length), let text else { return 0 }
        sink.insert(String(decoding: UnsafeBufferPointer(start: text, count: Int(count)), as: UTF16.self), NSRange(location: at, length: size))
    case 4: sink.render()
    case 5: sink.clearMarked()
    default: return 0
    }
    return 1
}

private typealias NativeCompositionHost = @convention(c) (UnsafeMutableRawPointer?, UInt32, UInt64, UInt64, UInt32) -> Int32
private final class NativeCompositionSink {
    let mark: (Int, NSRange) -> Void
    let publish: (Int) -> Void
    init(mark: @escaping (Int, NSRange) -> Void, publish: @escaping (Int) -> Void) { self.mark = mark; self.publish = publish }
}
private let nativeCompositionHost: NativeCompositionHost = { context, operation, location, length, style in
    guard let context, let at = Int(exactly: location), let size = Int(exactly: length) else { return 0 }
    let sink = Unmanaged<NativeCompositionSink>.fromOpaque(context).takeUnretainedValue()
    switch operation {
    case 0:
        guard style <= 2 else { return 0 }
        sink.mark(Int(style), NSRange(location: at, length: size))
    case 1: sink.publish(at)
    default: return 0
    }
    return 1
}

private typealias NativeCandidatesHost = @convention(c) (UnsafeMutableRawPointer?, UInt32, Int64) -> Int32
private final class NativeCandidatesSink {
    let operation: (UInt32, Int) -> Int32
    init(_ operation: @escaping (UInt32, Int) -> Int32) { self.operation = operation }
}
private let nativeCandidatesHost: NativeCandidatesHost = { context, operation, argument in
    guard let context, let value = Int(exactly: argument) else { return -1 }
    return Unmanaged<NativeCandidatesSink>.fromOpaque(context).takeUnretainedValue().operation(operation, value)
}

private typealias NativeMeaningCallback = @convention(c) (UnsafeMutableRawPointer?) -> Void
private final class NativeMeaningSink {
    let action: () -> Void
    init(_ action: @escaping () -> Void) { self.action = action }
}
private let nativeMeaningCallback: NativeMeaningCallback = { context in
    guard let context else { return }
    let sink = Unmanaged<NativeMeaningSink>.fromOpaque(context).takeRetainedValue()
    sink.action()
}

typealias NativeModeServiceHost = @convention(c) (UnsafeMutableRawPointer?, UnsafeMutableRawPointer?, UInt32, Int32) -> Int32

typealias NativeIndicatorEnabled = @convention(c) () -> Int32

private typealias NativeDictionaryDirectory = @convention(c) (UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer?
private final class NativeDictionaryDirectorySink {
    let directory:() -> String?
    init(_ directory:@escaping () -> String?) {self.directory=directory}
}
private let nativeDictionaryDirectory:NativeDictionaryDirectory = {context in
    guard let context,
          let value=Unmanaged<NativeDictionaryDirectorySink>.fromOpaque(context).takeUnretainedValue().directory() else {return nil}
    return Unmanaged.passRetained(value as NSString).toOpaque()
}
