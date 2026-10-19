//! The gallery's navigation history. A place is a category and page pair;
//! the user's own page changes are visits, and back, forward and the history
//! list reveal a place again without recording it.
use pert_ui::*;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

/// A place in the gallery: a category key and a page key.
pub(crate) type Place = (String, String);

/// The gallery's history and the bookkeeping that turns tab selections into
/// places. Clones share it.
#[derive(Clone)]
pub(crate) struct GalleryHistory(Rc<Inner>);

struct Inner {
    history: NavigationHistory<Place>,
    /// Titles of categories and pages by key; page keys are unique across
    /// the gallery and category keys end in `-category`.
    titles: RefCell<HashMap<String, String>>,
    /// Page keys registered since the last category, in order.
    pending: RefCell<Vec<String>>,
    /// Each category's first page, in category order.
    first: RefCell<Vec<(String, String)>>,
    /// The page each category's strip shows.
    shown: RefCell<HashMap<String, String>>,
    /// The strips, resolved after mount.
    strips: RefCell<Option<Strips>>,
}

struct Strips {
    top: WidgetId,
    pages: HashMap<String, WidgetId>,
}

impl GalleryHistory {
    pub(crate) fn new(ui: &Runtime) -> Result<Self, RuntimeError> {
        Ok(Self(Rc::new(Inner {
            history: NavigationHistory::new(ui)?,
            titles: RefCell::default(),
            pending: RefCell::default(),
            first: RefCell::default(),
            shown: RefCell::default(),
            strips: RefCell::default(),
        })))
    }

    pub(crate) fn history(&self) -> NavigationHistory<Place> {
        self.0.history.clone()
    }

    /// A page tab whose title the history labels its visits with.
    pub(crate) fn page(&self, key: &str, title: &str, content: Element) -> Tab {
        self.0
            .titles
            .borrow_mut()
            .insert(key.to_owned(), title.to_owned());
        self.0.pending.borrow_mut().push(key.to_owned());
        Tab::new(key, title, content)
    }

    /// A top-level category: a fixed tab whose content is its own detachable
    /// strip of the pages registered through [`Self::page`] since the last
    /// category. The strip reports its selections to the history.
    pub(crate) fn category(
        &self,
        key: &str,
        title: &str,
        pages: impl IntoIterator<Item = Tab>,
    ) -> Tab {
        let registered = std::mem::take(&mut *self.0.pending.borrow_mut());
        if let Some(first) = registered.first() {
            self.0
                .first
                .borrow_mut()
                .push((key.to_owned(), first.clone()));
            self.0
                .shown
                .borrow_mut()
                .insert(key.to_owned(), first.clone());
        }
        self.0
            .titles
            .borrow_mut()
            .insert(key.to_owned(), title.to_owned());
        let history = self.clone();
        let category = key.to_owned();
        let strip = Tabs::new(pages)
            .detachable(true)
            .on_select(move |_, change| history.page_selected(&category, change));
        Tab::new(key, title, Element::new("category-pages", strip)).fixed()
    }

    /// Records a page change in `category`'s strip. Every change updates the
    /// page the category shows; only the user's own is a visit.
    fn page_selected(&self, category: &str, change: &SelectionChange) -> Result<(), RuntimeError> {
        let Some(page) = change.selected.clone() else {
            return Ok(());
        };
        self.0
            .shown
            .borrow_mut()
            .insert(category.to_owned(), page.clone());
        if change.origin == ChangeOrigin::User {
            self.record(&(category.to_owned(), page))?;
        }
        Ok(())
    }

    /// Records a category change in the top strip: the place is the page that
    /// category shows.
    pub(crate) fn category_selected(&self, change: &SelectionChange) -> Result<(), RuntimeError> {
        let Some(category) = change.selected.clone() else {
            return Ok(());
        };
        if change.origin != ChangeOrigin::User {
            return Ok(());
        }
        let page = self.0.shown.borrow().get(&category).cloned();
        match page {
            Some(page) => self.record(&(category, page)),
            None => Ok(()),
        }
    }

    /// Records a visit to `place`, labelled "Category › Page".
    fn record(&self, place: &Place) -> Result<(), RuntimeError> {
        let label = self.label(place);
        self.0.history.visit(place.clone(), label).map(|_| ())
    }

    /// Where a component at the addressable key `path` sits, as
    /// "Category › Page", for a component on a gallery page; `None` for the
    /// menu bar, dialogs and other content outside the pages.
    pub(crate) fn location(&self, path: &[String]) -> Option<String> {
        let at = path
            .iter()
            .position(|segment| segment == "category-pages")?;
        let category = path.get(at.checked_sub(1)?)?;
        let page = path.get(at + 1)?;
        Some(self.label(&(category.clone(), page.clone())))
    }

    fn label(&self, (category, page): &Place) -> String {
        let titles = self.0.titles.borrow();
        let title = |key: &String| titles.get(key).cloned().unwrap_or_else(|| key.clone());
        format!("{} › {}", title(category), title(page))
    }

    /// Resolves the strips after mount and records the first place shown, so
    /// the first page change has somewhere to go back to.
    pub(crate) fn start(&self, ui: &Runtime, root: &WidgetId) -> Result<(), AppError> {
        let top = crate::gallery::gallery_tabs(ui, root)?;
        let mut pages = HashMap::new();
        for (category, _) in self.0.first.borrow().iter() {
            // A page's own content may reuse a key; the address names the
            // category's strip exactly.
            let strip = crate::gallery::find_at(ui, &top, &[category.as_str(), "category-pages"])?;
            pages.insert(category.clone(), strip);
        }
        *self.0.strips.borrow_mut() = Some(Strips { top, pages });
        let first = self.0.first.borrow().first().cloned();
        if let Some(place) = first {
            self.record(&place)?;
        }
        Ok(())
    }

    /// Shows `place`: its category's tab, then its page, each selected by key
    /// on its strip. The strips report these selections as reveals, so they
    /// are not recorded. A refusal is an error naming the strip that refused.
    pub(crate) fn show(
        &self,
        ui: &mut Runtime,
        (category, page): &Place,
    ) -> Result<(), RuntimeError> {
        let (top, strip) = {
            let strips = self.0.strips.borrow();
            let strips = strips.as_ref().ok_or(RuntimeError::Unmounted)?;
            let strip = strips
                .pages
                .get(category)
                .cloned()
                .ok_or(RuntimeError::Unmounted)?;
            (strips.top.clone(), strip)
        };
        for (container, key) in [(&top, category.as_str()), (&strip, page.as_str())] {
            match ui.reveal_child(container, key)? {
                ChildReveal::AlreadyVisible | ChildReveal::Changed => {}
                refusal => {
                    return Err(ApplicationError::from(format!(
                        "Could not open {key}: {refusal:?}"
                    ))
                    .into());
                }
            }
        }
        Ok(())
    }

    /// Shows `place` from an event phase, after the event, and reports it in
    /// the status bar. The operation is bound to the window rather than the
    /// widget, since choosing an entry closes the list it was chosen from.
    pub(crate) fn restore_later(
        &self,
        cx: &mut EventContext,
        entry: &HistoryEntry<Place>,
        status: &Signal<String>,
    ) -> Result<(), RuntimeError> {
        let (history, place, status) = (self.clone(), entry.value().clone(), status.clone());
        let label = entry.label().to_owned();
        cx.defer_detached(move |ui| {
            history.show(ui, &place)?;
            status.set(format!("Returned to {label}."))
        })
        .map(|_| ())
    }

    /// Goes back or forward and shows the entry, for a command action.
    pub(crate) fn step(
        &self,
        ui: &mut Runtime,
        back: bool,
        status: &Signal<String>,
    ) -> Result<(), RuntimeError> {
        let entry = if back {
            self.0.history.back()?
        } else {
            self.0.history.forward()?
        };
        let Some(entry) = entry else {
            return Ok(());
        };
        self.show(ui, entry.value())?;
        status.set(format!("Returned to {}.", entry.label()))
    }

    /// Opens a "Go to" destination and records it as a visit.
    pub(crate) fn go_to(
        &self,
        ui: &mut Runtime,
        category: &str,
        page: &str,
    ) -> Result<(), RuntimeError> {
        let place = (category.to_owned(), page.to_owned());
        self.show(ui, &place)?;
        self.record(&place)
    }
}
