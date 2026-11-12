//
//  RootViewController.swift
//  Dado Player — full-screen container that owns the native video layer and
//  reports the environment (safe area, keyboard, appearance, scale) to Rust.
//
//  winit installs its own root view controller for the egui surface, so this
//  controller never becomes the window's root in the normal case: it is added
//  as a child of whatever root controller exists and its view is inserted at
//  index 0, i.e. BEHIND the egui surface. Rust only ever pushes data *to* the
//  host through dado_host_*; the host only ever pushes data *to* Rust through
//  dado_app_set_env. No C-callable handle to this class is needed.
//
//  Two assumptions worth knowing, both unverifiable without a device:
//
//  1. Stacking. winit's root controller view may itself be the Metal/GL surface
//     (winit 0.29 uses a CAMetalLayer-backed UIView as the controller's view).
//     Inserting this controller's view at index 0 of that view puts us below its
//     *subviews* but above its own layer content. If the video ends up covering
//     the UI instead of sitting under it, the fix is to parent this controller's
//     view to the window instead of to winit's view.
//  2. Opacity. For the video to be visible at all, the egui surface must not
//     paint an opaque quad over the video rect (Android solves this with overlay
//     cutouts). A fully black screen during playback is a Rust-side clear-colour
//     issue, not a host issue.
//

import UIKit
import DadoBridge

final class RootViewController: UIViewController {

    /// Rust cannot reach this object (it only pushes env), but HostBridge can,
    /// and it is the anchor the video view is inserted into.
    static private(set) var shared: RootViewController?

    /// Last environment pushed to Rust. dado_app_set_env is cheap and idempotent,
    /// but viewDidLayoutSubviews fires a lot; skip identical pushes.
    private struct Env: Equatable {
        var top: Double
        var bottom: Double
        var left: Double
        var right: Double
        var ime: Double
        var dark: Bool
        var scale: Double
        var width: Double
        var height: Double
    }

    private var lastEnv: Env?
    private var keyboardOverlap: CGFloat = 0

    // MARK: - Installation

    /// Creates the controller on first use and parents it to the window's
    /// existing root controller (winit's), keeping this view at the bottom of
    /// the stack. Must be called on the main thread.
    @discardableResult
    static func install(in window: UIWindow) -> RootViewController {
        let controller = shared ?? RootViewController()
        shared = controller
        controller.attachIfNeeded(in: window)
        return controller
    }

    private func attachIfNeeded(in window: UIWindow) {
        guard view.superview == nil else { return }

        if let root = window.rootViewController, root !== self {
            // Normal path: winit already installed a root controller. Become its
            // child and sit underneath its content.
            root.addChild(self)
            view.frame = root.view.bounds
            view.autoresizingMask = [.flexibleWidth, .flexibleHeight]
            root.view.insertSubview(view, at: 0)
            didMove(toParent: root)
        } else if window.rootViewController == nil {
            // Early launch: nobody owns the window yet. Take it, winit will not
            // replace an existing root controller's view hierarchy.
            window.rootViewController = self
        }

        view.setNeedsLayout()
        pushEnv()
    }

    // MARK: - Lifecycle

    override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = .clear
        view.isOpaque = false
        view.autoresizingMask = [.flexibleWidth, .flexibleHeight]

        let center = NotificationCenter.default
        center.addObserver(self,
                           selector: #selector(keyboardWillChangeFrame(_:)),
                           name: UIResponder.keyboardWillChangeFrameNotification,
                           object: nil)
        center.addObserver(self,
                           selector: #selector(keyboardWillHide(_:)),
                           name: UIResponder.keyboardWillHideNotification,
                           object: nil)
    }

    deinit {
        NotificationCenter.default.removeObserver(self)
    }

    override func viewDidLayoutSubviews() {
        super.viewDidLayoutSubviews()
        pushEnv()
    }

    override func traitCollectionDidChange(_ previousTraitCollection: UITraitCollection?) {
        super.traitCollectionDidChange(previousTraitCollection)
        // Dark/light switch, Dynamic Type category changes and display scale
        // changes all land here.
        pushEnv()
    }

    // MARK: - Keyboard (IME inset)

    @objc private func keyboardWillChangeFrame(_ note: Notification) {
        guard let end = note.userInfo?[UIResponder.keyboardFrameEndUserInfoKey] as? NSValue else {
            return
        }
        // The notification carries a window-coordinate frame; convert into this
        // view's space so rotation and safe areas are already accounted for.
        let frame = view.convert(end.cgRectValue, from: nil)
        let overlap = view.bounds.maxY - frame.minY
        keyboardOverlap = max(0, overlap)
        pushEnv()
    }

    @objc private func keyboardWillHide(_ note: Notification) {
        keyboardOverlap = 0
        pushEnv()
    }

    // MARK: - Environment push

    private func pushEnv() {
        guard isViewLoaded, view.window != nil else { return }

        let insets = view.safeAreaInsets
        // Header contract: `ime` is the keyboard overlap in points and is
        // already included in `bottom`. Same rule as the Android port:
        // bottom = max(safe-area bottom, IME overlap).
        let ime = Double(keyboardOverlap)
        let bottom = max(Double(insets.bottom), ime)
        let scale = Double(view.window?.screen.scale ?? UIScreen.main.scale)
        let dark = traitCollection.userInterfaceStyle == .dark

        let env = Env(top: Double(insets.top),
                      bottom: bottom,
                      left: Double(insets.left),
                      right: Double(insets.right),
                      ime: ime,
                      dark: dark,
                      scale: scale,
                      width: Double(view.bounds.width),
                      height: Double(view.bounds.height))

        guard env != lastEnv else { return }
        lastEnv = env

        dado_app_set_env(env.top, env.bottom, env.left, env.right,
                         env.ime, env.dark ? 1 : 0, env.scale,
                         env.width, env.height)
    }
}
