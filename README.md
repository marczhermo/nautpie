# NautPie: DeployNaut API Console Client

**Goal**: Provides unified way of communicating with SilverStripe Platform DeployNaut API on the command line.

**Usage**: This is intended to be used for CI/CD or Continuous Integration and Delivery.

**Supported Clients**: Bitbucket Pipelines

**To Do**: CircleCI and GitLab



## DeployNaut API

Create a deployment with Git SHA
```
./nautpie.phar deploy:naut createDeployment --stack=example --environment=teststack1 --ref=40char-sha --ref_type=sha --bypass_and_start=true
```

Calls Git Fetch

```
./nautpie.phar deploy:naut gitFetch --stack=example
```

Collection of Previous Deployments

```
./nautpie.phar deploy:naut getDeployments --stack=example --environment=teststack1
```

Last Deployment Details

```
./nautpie.phar deploy:naut lastDeployment --stack=example --environment=teststack1
```
