// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
import XCTest

final class AppUITests: XCTestCase {
    private func capture(_ name: String) {
        let attachment = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        attachment.name = name; attachment.lifetime = .keepAlways
        add(attachment)
    }
    func testFlickGesturesAndVoicingInDemo() {
        let app = XCUIApplication()
        app.launch(); app.swipeUp()
        let demo = app.buttons["open-flick-demo"]
        XCTAssertTrue(demo.waitForExistence(timeout: 10)); demo.tap()
        let ka = app.buttons["flick-か"]
        XCTAssertTrue(ka.waitForExistence(timeout: 5))
        for offset in [CGVector(dx: -30, dy: 0), CGVector(dx: 0, dy: -30), CGVector(dx: 30, dy: 0), CGVector(dx: 0, dy: 30)] {
            let start = ka.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5))
            start.press(forDuration: 0.05, thenDragTo: start.withOffset(offset))
        }
        ka.tap()
        app.buttons["flick-は"].tap()
        let variant = app.buttons["濁点・半濁点・小文字を切替"]
        variant.tap(); variant.tap()
        XCTAssertEqual(app.staticTexts["preedit"].label, "きくけこかぱ")
        capture("flick-keyboard-light")
        app.buttons["確定 ↵"].tap()
        XCTAssertEqual(app.textViews["flick-demo-output"].value as? String, "きくけこかぱ")
        app.buttons["ABC・QWERTYに切替"].tap()
        XCTAssertTrue(app.buttons["q"].exists)
        app.buttons["かなフリックに切替"].tap()
        XCTAssertTrue(app.buttons["flick-あ"].exists)
    }
    private func nativeMenuAction(_ names: [String], in app: XCUIApplication) {
        for _ in 0..<8 {
            for name in names {
                let button = app.buttons[name]
                if button.exists && button.isHittable { button.tap(); return }
                let item = app.menuItems[name]
                if item.exists && item.isHittable { item.tap(); return }
            }
            let next = app.buttons.matching(NSPredicate(format: "label IN %@", ["Forward", "Next", "More", "Show more items", "次へ", "その他", "次のメニュー項目を表示"])).firstMatch
            guard next.exists else { break }
            next.tap()
        }
        XCTFail("Native menu item missing: " + names.joined(separator: ", "))
    }
    func testSelectedTextConversionAndDeletion() {
        let app = XCUIApplication(); app.launch(); app.swipeUp()
        app.buttons["open-flick-demo"].tap()
        XCTAssertTrue(app.buttons["flick-な"].waitForExistence(timeout: 10))
        XCTAssertFalse(app.buttons["keyboard-edit-toggle"].exists)
        XCTAssertFalse(app.buttons["demo-select-all"].exists)
        let keys: [(String, CGVector)] = [("flick-な", CGVector(dx: -30, dy: 0)), ("flick-は", CGVector(dx: 0, dy: 30)), ("flick-わ", CGVector(dx: 0, dy: -30)), ("flick-か", CGVector(dx: 0, dy: 30))]
        for (name, offset) in keys {
            let start = app.buttons[name].coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5))
            start.press(forDuration: 0.05, thenDragTo: start.withOffset(offset))
        }
        app.buttons["濁点・半濁点・小文字を切替"].tap()
        app.buttons["確定 ↵"].tap()
        let output = app.textViews["flick-demo-output"]
        XCTAssertEqual(output.value as? String, "にほんご")
        output.doubleTap()
        XCTAssertTrue(app.menuItems["Meltypeで変換"].waitForExistence(timeout: 5))
        capture("native-selection-menu")
        nativeMenuAction(["Meltypeで変換"], in: app)
        // UIKit selects 「にほん」 within 「にほんご」: preserve the unselected suffix.
        XCTAssertTrue(app.sheets.buttons["ニホン"].waitForExistence(timeout: 5))
        XCTAssertEqual(output.value as? String, "にほんご")
        app.sheets.buttons["ニホン"].tap()
        XCTAssertEqual(output.value as? String, "ニホンご")
    }
    func testNativeCopyDeleteAndPaste() {
        let app = XCUIApplication(); app.launch(); app.swipeUp()
        let output = app.textViews["input-test"]
        output.tap(); output.typeText("hello")
        output.doubleTap()
        nativeMenuAction(["Copy", "コピー"], in: app)
        output.doubleTap()
        nativeMenuAction(["選択範囲を削除"], in: app)
        XCTAssertEqual(output.value as? String, "")
        output.press(forDuration: 1.0)
        nativeMenuAction(["Paste", "ペースト", "貼り付け"], in: app)
        XCTAssertEqual(output.value as? String, "hello")
    }
    func testHomeInputUsesNativeMeltypeMenu() {
        let app = XCUIApplication(); app.launch(); app.swipeUp()
        let input = app.textViews["input-test"]
        input.tap(); input.typeText("nihongo")
        input.doubleTap()
        nativeMenuAction(["Meltypeで変換"], in: app)
        XCTAssertTrue(app.sheets.buttons["ニホンゴ"].waitForExistence(timeout: 5))
        app.sheets.buttons["ニホンゴ"].tap()
        XCTAssertEqual(input.value as? String, "ニホンゴ")
    }
    func testSetupAndAppearanceScreens() {
        let app = XCUIApplication()
        app.launchArguments = ["-AppleLanguages", "(ja)", "-AppleLocale", "ja_JP"]
        app.launch()
        XCTAssertTrue(app.buttons["open-settings"].waitForExistence(timeout: 10))
        capture("home-light")
        app.navigationBars.buttons["背景・テーマ"].tap()
        XCTAssertTrue(app.buttons["choose-background"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["ライト"].exists)
        XCTAssertTrue(app.buttons["ダーク"].exists)
        capture("appearance-light")
        app.navigationBars.buttons.element(boundBy: 0).tap()
        let input = app.textViews["input-test"]
        app.swipeUp()
        XCTAssertTrue(input.waitForExistence(timeout: 5))
        input.tap()
        input.typeText("Hello Meltype")
        XCTAssertEqual(input.value as? String, "Hello Meltype")
        capture("input-light")
    }
}
