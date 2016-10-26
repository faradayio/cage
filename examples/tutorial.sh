#!/bin/bash
#
# This script contains all the commands used in the standard `cage`
# tutorials, and it can be used as a test suite to verify that they work.

# Fail immediately if any error occurs.
set -euo pipefail

# Print the commands we run.
set -o xtrace

# Create a new project and switch to it.
cage new tutorial
cd tutorial

# Start the new project running.
cage up --init
cage status

# Mount a directory.
cage source ls
cage source mount rails_hello
cage up
cage status

# Test various other commands occasionally used in tutorials.  We haven't
# figure out how to test `cage shell web` yet.
cage test web
cage run rake -T

# Get any annoying files created with another user ID inside the container.
cage exec web rm -rf ./tmp

# Shut down our application and clean up.
cage stop
cage rm -fv
docker network rm tutorial_default
docker volume rm tutorial_db
cd ..
rm -rf tutorial
