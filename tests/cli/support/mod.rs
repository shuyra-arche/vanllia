use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct Fixture {
    pub root: PathBuf,
}

impl Fixture {
    pub fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "vanllia-discovery-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
        Self { root }
    }

    pub fn write(&self, path: &str, text: &str) {
        let path = self.root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    pub fn project(&self) {
        self.write(
            "vanllia.toml",
            "schema = 1\nunits = ['probe']\n[boot]\nsystem = 'system'\n",
        );
        self.write(
            "Cargo.toml",
            "[workspace]\nresolver = '3'\nmembers = ['system', 'probe', 'unused']\n",
        );
        self.binary("system", "system");
        self.binary("probe", "probe");
        self.binary("unused", "unused");
    }

    pub fn binary(&self, directory: &str, id: &str) {
        self.write(&format!("{directory}/Cargo.toml"), &format!("[package]\nname = '{id}-server'\nversion = '0.1.0'\nedition = '2024'\n[[bin]]\nname = '{id}'\npath = 'src/main.rs'\n"));
        self.write(&format!("{directory}/src/main.rs"), "fn main() {}\n");
        self.write(
            &format!("{directory}/unit.toml"),
            &format!("schema = 1\nid = '{id}'\n[loader]\nimage = '{id}'\n"),
        );
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

pub fn git(root: &Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
