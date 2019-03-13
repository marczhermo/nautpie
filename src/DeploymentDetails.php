<?php

namespace Marcz\Phar\NautPie;

class DeploymentDetails
{
    use CheckHelper;

    protected $details = [
        'ref' => '',
        'ref_type' => 'sha',
        'title' => '[CI] Deployment',
        'summary' => '',
        'bypass' => false,
        'bypass_and_start' => false,
        'schedule_start_unix' => null,
        'schedule_end_unix' => null,
        'locked' => false,
    ];

    public function __construct($details = [])
    {
        $details = array_intersect_key($details, $this->details);

        $this->details = array_merge($this->details, $details);
    }

    public function values()
    {
        return array_filter($this->details, [$this, 'isNotNull']);
    }

    public function promoteFromUAT()
    {
        $this->details['ref'] = '';
        $this->details['ref_type'] = 'promote_from_uat';

        return $this;
    }

    public function redeploy($yes = false)
    {
        if ($this->checkBoolean($yes)) {
            $this->details['ref'] = '';
            $this->details['ref_type'] = 'redeploy';
        }

        return $this;
    }

    public function title($title)
    {
        $this->details['title'] = $title;

        return $this;
    }

    public function summary($summary)
    {
        $this->details['summery'] = $summary;

        return $this;
    }

    public function bypassAndStart($yes = true)
    {
        $this->details['bypass_and_start'] = $this->checkBoolean($yes);

        return $this;
    }

    public function scheduleToStart($timeString = 'now')
    {
        $this->details['schedule_start_unix'] = strtotime($timeString);

        return $this;
    }

    public function reference($ref)
    {
        $this->details['ref'] = $ref;

        return $this;
    }

    public function referenceType($type)
    {
        $this->details['ref_type'] = $type;

        return $this;
    }
}
