use std::path::PathBuf;

pub fn run_worker(absolute_tex_file_path: PathBuf) -> () {
    println!("{:?}", absolute_tex_file_path);

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
