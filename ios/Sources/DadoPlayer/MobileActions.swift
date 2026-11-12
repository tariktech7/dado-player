import UIKit
import DadoBridge

private func presentingController() -> UIViewController? {
    let windows = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }.flatMap { $0.windows }
    var controller = (windows.first { $0.isKeyWindow } ?? UIApplication.shared.windows.first { $0.isKeyWindow })?.rootViewController
    while let shown = controller?.presentedViewController { controller = shown }
    return controller
}

@_cdecl("dado_host_edit")
public func editField(_ id: UInt64, _ title: UnsafePointer<CChar>, _ initial: UnsafePointer<CChar>, _ kind: Int32) {
    let label = String(cString: title)
    let value = String(cString: initial)
    DispatchQueue.main.async {
        guard let root = presentingController(), !(root is UIAlertController) else { return }
        let editor = UIAlertController(title: label, message: nil, preferredStyle: .alert)
        editor.addTextField { field in
            field.text = value
            field.isSecureTextEntry = kind == 2
            field.keyboardType = kind == 1 ? .URL : .default
            field.autocapitalizationType = .none
            field.autocorrectionType = .no
            field.spellCheckingType = .no
            field.returnKeyType = .done
        }
        editor.addAction(UIAlertAction(title: "Abbrechen", style: .cancel))
        let save = UIAlertAction(title: "Übernehmen", style: .default) { _ in
            (editor.textFields?.first?.text ?? "").withCString { dado_app_edited(id, $0) }
        }
        editor.addAction(save)
        editor.preferredAction = save
        root.present(editor, animated: true) { editor.textFields?.first?.becomeFirstResponder() }
    }
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
