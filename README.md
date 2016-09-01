# Faraday Conductor: Orchestrates `docker-compose` for large, multi-pod apps

This is a work in progress using the
[`docker_compose`](https://github.com/emk/docker\_compose-rs) library.  It's
a reimplementation of our internal, _ad hoc_ tools using the new
`docker-compose.yml` version 2 format and Rust.

## Background

The standard docker and docker-compose tooling is designed around the docker
swarm product. In this envrionment, your apps run on a virtual network and can
all talk to each other using custom, docker-only hostnames that map across
hosts. 

The swarm approach can be problematic and a different way of organizing
microservices has been adopted by platforms like Kubernetes and Amazon ECS. That
is, the idea of organizing related services into "pods" (Kubernetes) or "tasks"
(ECS).

This tool aims to be the "rails for microservices," that is, an opinionated
framework for assembling services using the pod/task paradigm. Note: this
project uses the name "pod" for this concept.

### What is a pod?

A pod is a group of related services. If one service goes down, the entire pod
is taken down and a new pod is usually started by the orchestration system. Pods
make linking services to one another simpler because there is a guarantee that
all the services in a pod run on the same physical machine.

For simple apps, you may have only one pod, and that is just fine.

## Usage

### Creating a new project

`conductor new hello_world` command will create a new conductor repo called
`hello_world` with a minimal set of configuration files. Your new project will
have a `default` pod.

### Adding services

`conductor generate service db --image postgresql` will add a new service based solely
on a docker image.

`conductor generate service web --source git@github.com:myaccount/web.git --link db`
will add a new service called `web` that can run code cloned from the given repo
URL. With the `--link` parameter, you can specify that the web service links to
the db service. Conductor will assume you want to use a docker image named
`myaccount/web`.  You can override this with the `--image` option.

### Configuring service environments

By default there are three environments: `test`, `development`, and
`production`. You can set environment variables for each environment. Let's
configure our web service to talk to the db:
`conducctor config:set web development DATABASE_URL postgres://postgres@db/app`.

### Running the entire project

`conductor up` will bring up all pods. In our example we have one pod, default,
with two services, web and db, that will be brought up.

### Checking out a service

`conductor checkout web` will allow you to make code updates to the web service.
This is really this project's raison d'être. Once a project is checked out, code
from your locally cloned repo in `src/web` will be mounted into the running web
container. Now you can make local updates and have them appear instantly!

Note that we have to make some assumptions for this to work. First, the
project's Dockerfile must expect the code to be run from a `/app` directory.
Second any libraries that your app uses must be "baked into" the container
image.
