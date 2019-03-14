<?php
namespace Marcz\Phar\NautPie;

use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Output\OutputInterface;
use Symfony\Component\Console\Input\InputOption;
use Symfony\Component\Console\Output\ConsoleOutputInterface;
use Symfony\Component\Console\Output\StreamOutput;
trait InputOutputHelper
{
    protected $input;
    protected $output;
    protected $options = [];
    protected $stdErr;

    /**
     * Initializes the command after the input has been bound and before the input
     * is validated.
     *
     * This is mainly useful when a lot of commands extends one main command
     * where some things need to be initialized based on the input arguments and options.
     *
     * @see InputInterface::bind()
     * @see InputInterface::validate()
     */
    protected function initialize(InputInterface $input, OutputInterface $output)
    {
        $this->input = $input;
        $this->output = $output;

        // If it's available, get stdErr output
        if($output instanceof ConsoleOutputInterface) {
            $this->stdErr = $output->getErrorOutput();
        } else {
            $this->stdErr = new StreamOutput(fopen('php://stderr', 'w'));
        }

        $this->loadEnvironment();
        $this->curlSetup();
    }

    public function getOption($name)
    {
        return $this->input->getOption($name);
    }

    public function getOptions()
    {
        return array_intersect_key($this->input->getOptions(), $this->options);
    }

    public function setOptions($options, $defaultValue = null)
    {
        $this->options = $options;

        foreach ($this->options as $option => $description) {

            $this->addOption($option, null, InputOption::VALUE_OPTIONAL, $description, $defaultValue);
        }

        return $this->options;
    }

    public function warning($message)
    {
        $this->message($message, 'fg=red;bg=yellow;');
    }

    public function success($message)
    {
        $this->message($message, 'fg=black;bg=green;');
    }

    public function message($message, $template = 'info')
    {
        $message = is_array($message) ? var_export($message, 1) : $message;

        $this->stdErr->writeln(
            sprintf('<%s>  %s </>', $template, $message)
        );
    }

    public function checkRequiredOptions()
    {
        $args = [];

        foreach (func_get_args() as $param) {
            $value = $this->getOption($param);

            if (empty($value)) {
                throw new \Exception('[Required:Option] ' .$param. ' is missing.', 1);
            }

            $methodName = 'validate' . ucfirst(strtolower($param));
            if (method_exists($this, $methodName)) {
                call_user_func_array([$this, $methodName], [$value]);
            }

            $args[] = $value;
        }

        return $args;
    }

    public function validateCommit($commit)
    {
        if (strlen($commit) !== 40) {
            throw new \Exception('[Action:DeployPackage] Requires 40-char commit', 1);
        }
    }
}
