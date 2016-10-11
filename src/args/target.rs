//! Pod and service arguments to `docker-compose.yml`.

use compose_yml::v2 as dc;

use errors::*;
use ovr::Override;
use pod::Pod;
use project::Project;

/// The pod and service within which to execute a command.  The lifetime
/// `'a` needs to be longer than the useful lifetime of this `Target`.
#[derive(Debug)]
pub struct Target<'a> {
    /// The override we're using to run this command.
    ovr: &'a Override,
    /// The name of the pod in which to run the command.
    pod: &'a Pod,
    /// The name of the service in which to run the command.
    service_name: &'a str,
    /// The `Service` object for the service where we'll run the command.
    service: dc::Service,
}

impl<'a> Target<'a> {
    /// Create a new `Target`, looking up the underlying pod and service
    /// objects.
    pub fn new(project: &'a Project,
               ovr: &'a Override,
               pod_name: &'a str,
               service_name: &'a str)
               -> Result<Target<'a>> {
        let pod = try!(project.pod(pod_name)
            .ok_or_else(|| err!("Cannot find pod {}", pod_name)));
        let file = try!(pod.merged_file(ovr));
        let service = try!(file.services
            .get(service_name)
            .ok_or_else(|| err!("Cannot find service {}", service_name)));
        Ok(Target {
            ovr: ovr,
            pod: pod,
            service_name: service_name,
            service: service.to_owned(),
        })
    }

    /// The active override for the command we want to run.
    pub fn ovr(&self) -> &Override {
        self.ovr
    }

    /// The pod for this target.
    pub fn pod(&self) -> &Pod {
        self.pod
    }

    /// The service name for this target.
    pub fn service_name(&self) -> &str {
        self.service_name
    }

    /// The `Service` object for this target.
    pub fn service(&self) -> &dc::Service {
        &self.service
    }
}
