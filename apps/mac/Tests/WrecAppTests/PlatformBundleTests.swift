import Foundation
import Testing

@testable import wrec_app

struct PlatformBundleTests {
    // Bundle.main.bundleURL is NSURL-bridged, where deleting the last path
    // component of "/" yields "/.." instead of "/" — the walk must still
    // terminate for a binary that lives outside any .app bundle.
    @Test func bareBinaryOutsideABundleResolvesToNil() {
        let bare = NSURL(fileURLWithPath: "/tmp/build/debug/wrec-app") as URL
        #expect(Platform.appBundle(containing: bare) == nil)
    }

    @Test func bundledExecutableResolvesItsAppRoot() {
        let exe = NSURL(fileURLWithPath: "/Applications/Wrec.app/Contents/MacOS/wrec-app") as URL
        #expect(Platform.appBundle(containing: exe)?.lastPathComponent == "Wrec.app")
    }

    @Test func rootItselfResolvesToNil() {
        let root = NSURL(fileURLWithPath: "/") as URL
        #expect(Platform.appBundle(containing: root) == nil)
    }
}
