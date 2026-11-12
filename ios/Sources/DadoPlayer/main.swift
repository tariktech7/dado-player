//
//  main.swift
//  Dado Player — iOS host entry point.
//
//  SwiftPM executables with a file named `main.swift` run top-level code, and
//  that is the only correct entry shape for this app. DadoBridge.h is explicit
//  about it (`dado_app_start`): Rust/eframe/winit calls `UIApplicationMain`
//  itself and takes over the main thread. A SwiftUI `@main App` would install a
//  second owner of UIApplicationMain and the two would fight over the run loop,
//  so there is no @main type and no SwiftUI here.
//

import Foundation
import DadoBridge

let fileManager = FileManager.default

if let appSupport = fileManager.urls(for: .applicationSupportDirectory,
                                     in: .userDomainMask).first {
    // iOS does not create Application Support inside the sandbox. The Rust core
    // points `directories` at this path for profiles.json / settings.json, so it
    // must exist before dado_app_start.
    do {
        try fileManager.createDirectory(at: appSupport,
                                        withIntermediateDirectories: true)
    } catch {
        // Non-fatal: the app still runs, it just cannot persist profiles.
        NSLog("%@", "Dado: cannot create Application Support (\(error.localizedDescription))")
    }

    // withCString keeps the buffer alive for the duration of the call; the callee
    // copies it (see header: "no pointer passed across this boundary stays valid
    // after the call returns").
    appSupport.path.withCString { dado_app_set_storage_dir($0) }
} else {
    NSLog("%@", "Dado: no Application Support directory in the sandbox")
}

// Does not return under normal operation: winit owns the UIKit run loop from
// here on. Must be called exactly once, after dado_app_set_storage_dir.
dado_app_start()
