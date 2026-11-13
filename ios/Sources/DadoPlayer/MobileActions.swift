import UIKit
import DadoBridge

private func presentingController() -> UIViewController? {
    let windows = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }.flatMap { $0.windows }
    var controller = (windows.first { $0.isKeyWindow } ?? UIApplication.shared.windows.first { $0.isKeyWindow })?.rootViewController
    while let shown = controller?.presentedViewController { controller = shown }
    return controller
}

private final class InlineEditor: NSObject, UITextFieldDelegate {
    static let shared = InlineEditor()
    private(set) var field: UITextField?
    private var id: UInt64 = 0

    func update(id: UInt64, label: String, value: String, kind: Int32, rect: CGRect, mode: Int32, dark: Bool) {
        if mode < 0 {
            if id == self.id || id == 0 { close() }
            return
        }
        guard let root = presentingController()?.view else { return }
        if mode == 1 && (self.id != id || field == nil) {
            close()
            self.id = id
            let input = UITextField()
            input.delegate = self
            input.accessibilityLabel = label
            input.text = value
            input.keyboardType = kind == 1 ? .URL : .default
            input.autocapitalizationType = .none
            input.autocorrectionType = .no
            input.spellCheckingType = .no
            input.font = .systemFont(ofSize: 16)
            input.returnKeyType = kind == 2 ? .done : .next
            input.addTarget(self, action: #selector(changed), for: .editingChanged)
            field = input
            root.addSubview(input)
        }
        guard self.id == id, let input = field else { return }
        input.overrideUserInterfaceStyle = dark ? .dark : .light
        input.backgroundColor = .secondarySystemBackground
        input.textColor = .label
        if input.isSecureTextEntry != (kind == 2) { input.isSecureTextEntry = kind == 2 }
        input.layer.cornerRadius = 8
        input.frame = rect
        if mode == 1 { input.becomeFirstResponder() }
    }

    @objc private func changed() {
        (field?.text ?? "").withCString { dado_app_edited(id, $0) }
    }

    func textFieldShouldReturn(_ textField: UITextField) -> Bool {
        let current = id
        let next = textField.returnKeyType == .next
        close()
        dado_app_edit_finished(current, next ? 1 : 0)
        return false
    }

    func close() {
        guard let input = field else { return }
        changed()
        field = nil
        input.resignFirstResponder()
        input.removeFromSuperview()
        id = 0
    }
}

@_cdecl("dado_host_edit")
public func editField(_ id: UInt64, _ title: UnsafePointer<CChar>, _ initial: UnsafePointer<CChar>, _ kind: Int32,
                      _ x: Double, _ y: Double, _ w: Double, _ h: Double, _ mode: Int32, _ dark: Int32) {
    InlineEditor.shared.update(id: id, label: String(cString: title), value: String(cString: initial), kind: kind,
                              rect: CGRect(x: x, y: y, width: w, height: h), mode: mode, dark: dark != 0)
}

@_cdecl("dado_host_mobile_update")
public func mobileUpdate(_ install: Int32) {
    DispatchQueue.main.async {
        if install == 0 {
            UIApplication.shared.open(URL(string: "https://github.com/tariktech7/dado-player/releases/latest")!)
            return
        }
        guard let root = presentingController() else { return }
        let file = FileManager.default.temporaryDirectory.appendingPathComponent("Dado-Player.ipa")
        let share = UIActivityViewController(activityItems: [file], applicationActivities: nil)
        if let pop = share.popoverPresentationController {
            pop.sourceView = root.view
            pop.sourceRect = CGRect(x: root.view.bounds.midX, y: root.view.bounds.midY, width: 1, height: 1)
        }
        root.present(share, animated: true)
    }
}
