import React, { useState, useEffect } from 'react';
import { createStyles, makeStyles, Theme } from '@material-ui/core/styles';
import Grid from '@material-ui/core/Grid';
import Button from '@material-ui/core/Button';
import DeleteIcon from '@material-ui/icons/Delete';
import RefreshIcon from '@material-ui/icons/Refresh';
import Typography from '@material-ui/core/Typography';
import List from '@material-ui/core/List';
import FailedRow, { FailedJob } from './failedRow';

interface FailedJobs {
  jobs: Array<FailedJob>,
  total_failed: number,
}

const stepBy = 10;

const useStyles = makeStyles((theme: Theme) =>
  createStyles({
    tabContent: {
      marginTop: theme.spacing(2),
    },
    gridRoot: {
      flexGrow: 1,
    },
    spaceLeft: {
      marginLeft: theme.spacing(5),
    }
  }),
);


const FailedContent: React.FC = () => {
  const [failedJobs, setJobs] = useState<FailedJobs>({ total_failed: 0, jobs: [] });
  const [wasReset, triggerReset] = useState(false);
  const [firstJob, setFirstJob] = useState(0);
  const [isLoading, setLoading] = useState(false);

  const handleNextClicked = () => {
    setFirstJob(firstJob + stepBy);
  }

  const handlePrevClicked = () => {
    setFirstJob(Math.max(0, firstJob - stepBy));
  }

  const handleQueueClear = () => {
    fetch('failed', {
      method: 'DELETE'
    }).then((response) => {
      if (response.ok) {
        setFirstJob(0);
        triggerReset(!wasReset);
      }
    })
  }

  const handleJobDeleted = (id: string) => {
    const filteredJobs = failedJobs.jobs.filter(job => job.payload.args[0].job_id !== id);
    setJobs({ total_failed: failedJobs.total_failed - 1, jobs: filteredJobs });
  }

  const handleRetry = () => {
    fetch('retry_all', {
      method: 'POST'
    }).then((response) => {
      if (response.ok) {
        setFirstJob(0);
        triggerReset(!wasReset);
      }
    })
  }

  const atMaxJob = () => firstJob + stepBy >= failedJobs.total_failed || failedJobs.total_failed === 0;
  const atMinJob = () => firstJob === 0;

  useEffect(() => {
    setLoading(true);
    fetch(`failed?from_job=${firstJob}`).then((response) => response.json()).then((data) => {
      setLoading(false);
      setJobs(data);
    });
  }, [firstJob, wasReset])

  const renderControls = () => {
    if (failedJobs.total_failed === 0) return null;
    return (
      <Grid container justify="center" spacing={2}>
        <Button onClick={handlePrevClicked} disabled={atMinJob() || isLoading}>Previous</Button>
        <Button disabled>Page {Math.ceil((firstJob + 1) / stepBy)}</Button>
        <Button onClick={handleNextClicked} disabled={atMaxJob() || isLoading}>Next</Button>
      </Grid>
    );
  }

  const classes = useStyles();
  return (
    <div className={classes.tabContent}>
      <Typography variant="h6">
        {failedJobs.total_failed} jobs failed
        <Button className={classes.spaceLeft} color="secondary" onClick={handleQueueClear}>Clear Failed <DeleteIcon /></Button>
        <Button className={classes.spaceLeft} onClick={handleRetry}>Retry All <RefreshIcon /> </Button>
      </Typography>

      <Grid className={classes.gridRoot} container spacing={2}>
        <Grid item xs={12}>
          {renderControls()}
        </Grid>
        <Grid item xs={12}>
          <List>
            {failedJobs.jobs.map(item => <FailedRow job={item} onJobDeleted={handleJobDeleted} onJobRetried={handleJobDeleted} key={item.payload.args[0].job_id} />)}
          </List>
        </Grid>
      </Grid>
    </div>
  );
}

export default FailedContent;