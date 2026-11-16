//
//  AppDelegate.swift
//  Dado Player — minimal UIApplicationDelegate.
//
//  Rust drives UIApplicationMain (see DadoBridge.h, dado_app_start), so UIKit
//  needs *a* delegate class to be present in the binary. This is it.
//
//  IMPORTANT: it does not create a window. winit creates and owns its own
//  UIWindow + root view controller when the Rust event loop starts; creating
//  an extra key window here would steal winit input. VideoHost creates only a
//  non-key background decoder window after winit has installed its UI. So didFinishLaunching just returns
//  true and leaves window ownership entirely to Rust.
//

import UIKit

class AppDelegate: UIResponder, UIApplicationDelegate {

    /// Never assigned by this host. Present because UIApplicationDelegate
    /// declares it and some UIKit code paths probe for it.
    var window: UIWindow?

    func application(_ application: UIApplication,
                     didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?) -> Bool {
        // winit owns the window; see the file header. Do not add subviews here —
        // no window exists yet at this point in the launch sequence.
        return true
    }
}
