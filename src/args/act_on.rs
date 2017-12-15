//! Specifying the pods, services or both acted on by a command.

use std::iter::Filter;
use std::slice;

use errors::*;
use pod::{Pod, PodType};
use project::{PodOrService, Pods, Project};

/// The names of pods, services or both to pass to one of our commands.
#[derive(Debug)]
pub enum ActOn {
    /// Act upon all the pods and/or services associated with this project.
    All,
    /// Act on services except those defined in `Pod`s of type
    /// `PodType::Task`.
    AllExceptTasks,
    /// Act upon only the named pods and/or services.
    Named(Vec<String>),
}

impl ActOn {
    /// Iterate over the pods or services specified by this `ActOn` object.
    pub fn pods_or_services<'a>(&'a self, project: &'a Project) -> PodsOrServices<'a> {
        let state = match *self {
            ActOn::All => State::PodIter(project.pods()),
            ActOn::AllExceptTasks => {
                let iter =
                    project.pods().filter(all_except_tasks as fn(&&Pod) -> bool);
                State::FilteredPodIter(iter)
            }
            ActOn::Named(ref names) => State::NameIter(names.into_iter()),
        };
        PodsOrServices {
            project: project,
            state: state,
        }
    }
}

/// A filter function which excludes `PodType::Task` pods.  We could use an
/// inline closure for this, but it's annoying to stick Rust closures into
/// structs, because the types get too complicated.
fn all_except_tasks(pod: &&Pod) -> bool {
    pod.pod_type() != PodType::Task
}

/// Internal state for `PodsOrServices` iterator.
#[derive(Debug)]
#[cfg_attr(feature = "clippy", allow(enum_variant_names))]
enum State<'a> {
    /// This corresponds to `ActOn::All`.
    PodIter(Pods<'a>),
    /// This corresponds to `ActOn::All`.
    FilteredPodIter(Filter<Pods<'a>, fn(&&Pod) -> bool>),
    /// This corresponds to `ActOn::Named`.
    NameIter(slice::Iter<'a, String>),
}

/// An iterator over the pods or services specified by an `ActOn` value.
#[derive(Debug)]
pub struct PodsOrServices<'a> {
    /// The project with which we're associated.
    project: &'a Project,

    /// Our internal iteration state.
    state: State<'a>,
}

impl<'a> Iterator for PodsOrServices<'a> {
    type Item = Result<PodOrService<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.state {
            State::PodIter(ref mut iter) => {
                iter.next().map(|pod| Ok(PodOrService::Pod(pod)))
            }
            State::FilteredPodIter(ref mut iter) => {
                iter.next().map(|pod| Ok(PodOrService::Pod(pod)))
            }
            State::NameIter(ref mut iter) => if let Some(name) = iter.next() {
                Some(self.project.pod_or_service_or_err(name))
            } else {
                None
            },
        }
    }
}
