use super::*;

#[test]
fn desired_nofile_soft_limit_only_raises_when_possible() {
    assert_eq!(desired_nofile_soft_limit(1024, 524_288, 8192), Some(8192));
    assert_eq!(desired_nofile_soft_limit(8192, 524_288, 8192), None);
    assert_eq!(desired_nofile_soft_limit(1024, 4096, 8192), Some(4096));
}

#[cfg(unix)]
#[test]
fn spawn_detached_creates_new_session() {
    let parent_sid = unsafe { libc::getsid(0) };

    let mut cmd = std::process::Command::new("sh");
    cmd.arg("-c")
        .arg("sleep 10")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    let mut child = super::spawn_detached(&mut cmd).expect("spawn detached child");
    let child_sid = unsafe { libc::getsid(child.id() as libc::pid_t) };
    child.kill().expect("terminate detached child");
    child.wait().expect("reap detached child");

    assert_eq!(
        child_sid as u32,
        child.id(),
        "detached child should lead its own session"
    );
    assert_ne!(
        child_sid as i32, parent_sid,
        "detached child should not share parent session"
    );
}
