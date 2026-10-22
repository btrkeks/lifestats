import UIKit

/// UIScene lifecycle stub introduced as part of the iOS UIScene migration
/// (Apple is making scene support mandatory in upcoming iOS releases). The
/// app is intentionally single-window — `UIApplicationSupportsMultipleScenes`
/// is `false` in Info.plist — so user-visible behaviour is identical to the
/// pre-migration AppDelegate-only world.
///
/// The window itself is still loaded from `Main.storyboard` (referenced via
/// `UISceneStoryboardFile` in Info.plist), whose initial view controller is
/// `FlutterViewController`. iOS instantiates this delegate, attaches the
/// loaded window to it, and dispatches scene-lifecycle events here.
///
/// `flutter_local_notifications`, `app_links`, and the rest of the plugin
/// chain all continue to receive lifecycle events through `AppDelegate` —
/// nothing scene-specific needs to be wired here.
class SceneDelegate: UIResponder, UIWindowSceneDelegate {
    var window: UIWindow?
}
