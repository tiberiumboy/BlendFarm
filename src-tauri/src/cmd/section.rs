use super::project::Project;

trait Section {
    fn get_content() -> String;
}

impl Section for Project {
    fn get_content() -> String {
        "./project".to_owned()
    }
}
