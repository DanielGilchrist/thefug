mod history;

use crate::history::History;

fn main() {
  let history = History::default();
  history.get();
}
