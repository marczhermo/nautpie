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
