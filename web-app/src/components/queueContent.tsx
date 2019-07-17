import React, { useState, useEffect } from 'react';
import Typography from '@material-ui/core/Typography';
import Grid from '@material-ui/core/Grid';
import Button from '@material-ui/core/Button';
import { createStyles, makeStyles, Theme } from '@material-ui/core/styles';
import DeleteIcon from '@material-ui/icons/Delete';
import List from '@material-ui/core/List';
import ListItem from '@material-ui/core/ListItem';
import ListItemText from '@material-ui/core/ListItemText';
import Divider from '@material-ui/core/Divider';
import { QueueJob, renderArguments } from '../utils/jobArgs';

interface QueueJobs {
  total_jobs: Number,
  jobs: Array<QueueJob>,
}

interface ContentProp {
  queue: string,
}

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

function renderJobItem(item: QueueJob) {
  return (
    <React.Fragment key={item.args[0].job_id}>
      <ListItem alignItems="flex-start">
        <ListItemText
          primary={item.args[0].job_class}
          secondary={renderArguments(item.args[0].arguments)}
        />
      </ListItem>
      <Divider variant="fullWidth" component="li" />
    </React.Fragment>
  );
}

const QueueContent: React.FC<ContentProp> = ({ queue }) => {
  const [queueJobs, setJobs] = useState<QueueJobs>({ total_jobs: 0, jobs: [] });
  const [firstJob, setFirstJob] = useState(0);

  const handleNextClicked = () => {
    setFirstJob(firstJob + 10);
  }

  const handlePrevClicked = () => {
    setFirstJob(Math.max(0, firstJob - 10));
  }

  const handleQueueClear = () => {
    fetch(`queue/${queue}`, {
      method: 'DELETE'
    }).then((response) => {
      if (response.ok) {
        window.location.reload();
      }
    })
  }

  const atMaxJob = () => firstJob > queueJobs.total_jobs || queueJobs.total_jobs === 0;
  const atMinJob = () => firstJob === 0;

  useEffect(() => {
    fetch(`queue/${queue}?first_job=${firstJob}`).then((response) => response.json()).then((data) => setJobs(data));
  }, [firstJob, queue])

  const renderControls = () => {
    if (queueJobs.total_jobs === 0) return null;
    return (
      <Grid container justify="center" spacing={2}>
        <Button onClick={handlePrevClicked} disabled={atMinJob()}>Previous</Button>
        <Button disabled>Page {Math.ceil((firstJob + 1) / 10)}</Button>
        <Button onClick={handleNextClicked} disabled={atMaxJob()}>Next</Button>
      </Grid>
    );
  }

  const classes = useStyles();
  return (
    <div className={classes.tabContent}>
      <Typography variant="h6">
        {queue}: {queueJobs.total_jobs} jobs
        <Button className={classes.spaceLeft} color="secondary" onClick={handleQueueClear}>Clear {queue} <DeleteIcon /></Button>
      </Typography>
      <Grid className={classes.gridRoot} container spacing={2}>
        <Grid item xs={12}>
          {renderControls()}
        </Grid>
      </Grid>
      <List>
        {queueJobs.jobs.map(job => renderJobItem(job))}
      </List>
    </div>
  );
}

export default QueueContent;