use cursive::{view::Nameable, views::TextView, Cursive};
use intersect_core::documents::FragmentView;

use super::{subview, Panel};

pub struct FragmentPanel {
    pub view: FragmentView,
}

impl Panel for FragmentPanel {
    fn title(&self) -> String {
        "fragment".to_string()
    }

    // fragments are immutable
    fn has_updates(&self) -> bool {
        false
    }

    fn build_view(&mut self, id: usize) -> Box<dyn cursive::View> {
        Box::new(TextView::new(format!("{}", self.view)).with_name(subview(id, "content")))
    }

    fn make_update(&mut self, _id: usize) -> Box<dyn FnOnce(&mut Cursive)> {
        unreachable!("fragment panels never update")
    }
}
