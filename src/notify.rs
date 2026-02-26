pub fn send_completion(name: &str, duration_display: &str, silent: bool) {
    let mut notification = notify_rust::Notification::new();
    notification
        .summary(&format!("{name} complete"))
        .body(&format!("{duration_display} timer finished"))
        .appname("pomitik");

    #[cfg(target_os = "macos")]
    if !silent {
        notification.sound_name("Glass");
    }

    if let Err(e) = notification.show() {
        eprintln!("Failed to send notification: {e}");
    }
}
