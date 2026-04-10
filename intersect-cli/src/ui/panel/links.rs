use cursive::{view::Nameable, views::TextView, Cursive};
use intersect_core::{documents::LinksDocument, OpenDocument};

use super::{subview, Panel};

pub struct LinksPanel {
    pub doc: OpenDocument<LinksDocument>,
}

impl Panel for LinksPanel {
    fn title(&self) -> String {
        "links".to_string()
    }

    fn has_updates(&self) -> bool {
        self.doc.updates.has_changed().unwrap_or(false)
    }

    fn build_view(&mut self, id: usize) -> Box<dyn cursive::View> {
        // TODO: render links list once LinksDocument is implemented
        let _ = self.doc.updates.borrow_and_update();
        Box::new(TextView::new("(links not yet implemented)").with_name(subview(id, "content")))
    }

    fn make_update(&mut self, id: usize) -> Box<dyn FnOnce(&mut Cursive)> {
        // TODO: render links list once LinksDocument is implemented
        let _ = self.doc.updates.borrow_and_update();
        let name = subview(id, "content");
        Box::new(move |s| {
            s.call_on_name(&name, |v: &mut TextView| {
                v.set_content("(links not yet implemented)");
            });
        })
    }
}
