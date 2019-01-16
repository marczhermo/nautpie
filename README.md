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
./nautpie.phar deploy:naut deployments --stack=example --environment=teststack1 
```

Last Deployment Details

```
./nautpie.phar deploy:naut lastDeployment --stack=example --environment=teststack1 
```


## Bitbucket Pipelines

Create a deployment with Git SHA

```
./nautpie.phar ci:bitbucket deployGitSha --stack=example --environment=uat --commit=40char-sha
```

Create a deployment with packaged .tar.gz file

```
./nautpie.phar ci:bitbucket deployPackage --stack=example --environment=uat --commit=40char-sha
```

Create a Bitbucket access token

```
./nautpie.phar ci:bitbucket accessToken
```

## How to Call Other Commands

```
use Symfony\Component\Console\Input\ArrayInput;
// ...

$command = $this->getApplication()->find('deploy:naut');
$command->resetCurlData();
$command->setContentType('application/vnd.api+json');
$otherInput = new ArrayInput([
    'command' => 'deploy:naut',
    'action' => 'fetch'
    '--url' => 'project/hello/environment/testsite/deploys?title=Release+4.8.2',
]);
$returnCode = $command->run($otherInput, $output);
```

## Building Phar Executable

Run: `box build -v`