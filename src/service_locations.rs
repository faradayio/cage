//! Keeps track of what pods contain what services.

use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

use crate::pod::Pod;

/// Maps user-visible service names of the forms `pod_name/service_name` to
/// `(pod_name, service_name)` pairs, and also does the same for bare
/// `service_name` name values if they're unique.
///
/// This is a bit complicated, but it significantly improves the ergonomics
/// of many of our commands.
#[derive(Debug)]
pub struct ServiceLocations {
    /// Our location information.
    locations: BTreeMap<String, (String, String)>,
}

impl ServiceLocations {
    /// Build a new `ServiceLocations` object by inspecting the services
    /// provided by a list of pods.
    pub fn new(pods: &[Pod]) -> ServiceLocations {
        /// A local type to help us determine which names are unique.
        enum ShortNameStatus<'a> {
            /// We've only seen this service name in a single pod.
            UniqueInPod(&'a str),
            /// We've seen this service in more than one pod, but we
            /// don't care which.
            Duplicate,
        }

        // Iterate over our pods.
        let mut locations = BTreeMap::new();
        let mut short_names: BTreeMap<String, ShortNameStatus<'_>> = BTreeMap::new();
        for pod in pods {
            for service in pod.service_names() {
                // Add long names immediately as `pod/service`.
                locations.insert(
                    format!("{}/{}", pod.name(), service),
                    (pod.name().to_owned(), service.to_owned()),
                );

                // Keep track of short names to see if we have a unique
                // name `service`.
                match short_names.entry(service.to_owned()) {
                    Entry::Vacant(vacant) => {
                        vacant.insert(ShortNameStatus::UniqueInPod(pod.name()));
                    }
                    Entry::Occupied(mut occupied) => {
                        occupied.insert(ShortNameStatus::Duplicate);
                    }
                }
            }
        }

        // Add our unique short names.
        for (service, status) in short_names {
            if let ShortNameStatus::UniqueInPod(pod) = status {
                locations
                    .insert(service.to_owned(), (pod.to_owned(), service.to_owned()));
            }
        }

        ServiceLocations { locations }
    }

    /// Find a service by name.
    pub fn find(&self, service_name: &str) -> Option<(&str, &str)> {
        self.locations
            .get(service_name)
            .map(|(pod, service)| (&pod[..], &service[..]))
    }
}
