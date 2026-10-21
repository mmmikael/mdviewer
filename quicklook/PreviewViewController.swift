// Quick Look preview for Markdown files (space bar in Finder).
//
// Shows the file with MDViewer's own renderer: a web view loads the bundled
// preview page (Resources/web, built from src/quicklook) and hands it the
// file's text.

import Cocoa
import Quartz
import WebKit

@objc(MDVPreviewViewController)
final class PreviewViewController: NSViewController, QLPreviewingController, WKNavigationDelegate {
    private static let maxBytes = 5 * 1024 * 1024

    private var webView: WKWebView!
    private var pending: (source: String, baseDir: String, done: (Error?) -> Void)?

    override var nibName: NSNib.Name? { nil }

    override func loadView() {
        webView = WKWebView(frame: NSRect(x: 0, y: 0, width: 800, height: 600))
        webView.navigationDelegate = self
        view = webView
    }

    func preparePreviewOfFile(at url: URL, completionHandler handler: @escaping (Error?) -> Void) {
        var data = (try? Data(contentsOf: url)) ?? Data()
        if data.count > Self.maxBytes { data = data.prefix(Self.maxBytes) }
        var source = String(decoding: data, as: UTF8.self)
        if source.hasPrefix("\u{FEFF}") { source.removeFirst() }
        pending = (source, url.deletingLastPathComponent().path, handler)

        guard let web = Bundle.main.resourceURL?.appendingPathComponent("web") else {
            return finish(nil)
        }
        webView.loadFileURL(web.appendingPathComponent("preview.html"), allowingReadAccessTo: web)
    }

    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        guard let job = pending else { return }
        webView.callAsyncJavaScript(
            "await window.mdvRender(source, baseDir)",
            arguments: ["source": job.source, "baseDir": job.baseDir],
            in: nil,
            in: .page
        ) { [weak self] _ in self?.finish(nil) }
    }

    func webView(_ webView: WKWebView, didFail navigation: WKNavigation!, withError error: Error) {
        finish(error)
    }

    func webView(
        _ webView: WKWebView, didFailProvisionalNavigation navigation: WKNavigation!, withError error: Error
    ) {
        finish(error)
    }

    // Only the preview page itself may load; links can't navigate the preview.
    func webView(
        _ webView: WKWebView, decidePolicyFor navigationAction: WKNavigationAction,
        decisionHandler: @escaping (WKNavigationActionPolicy) -> Void
    ) {
        decisionHandler(navigationAction.request.url?.isFileURL == true && navigationAction.navigationType == .other ? .allow : .cancel)
    }

    private func finish(_ error: Error?) {
        guard let job = pending else { return }
        pending = nil
        job.done(error)
    }
}
