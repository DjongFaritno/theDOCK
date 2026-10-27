fn main() {
    println!("cargo:rerun-if-changed=assets/thedock.ico");

    #[cfg(windows)]
    {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/thedock.ico");
        resource.compile().expect("could not embed theDOCK icon");
    }
}
