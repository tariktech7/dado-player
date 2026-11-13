// Observes safe areas, keyboard overlap and appearance of the winit window.
// The noninteractive child view stays behind the Rust surface.
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
            view.frame = window.bounds
            view.autoresizingMask = [.flexibleWidth, .flexibleHeight]
            // Keep the environment observer behind the interactive Rust surface.
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
        view.isUserInteractionEnabled = false // Touches belong to the winit surface.
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
