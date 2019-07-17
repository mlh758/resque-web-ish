import React, { useEffect, useState } from 'react';
import { createStyles, makeStyles, Theme } from '@material-ui/core/styles';
import List from '@material-ui/core/List';
import ListItem from '@material-ui/core/ListItem';
import ListItemText from '@material-ui/core/ListItemText';
import Divider from '@material-ui/core/Divider';
import { renderArguments, QueueJob } from '../utils/jobArgs';
import { Typography } from '@material-ui/core';

interface Worker {
  id: string,
  payload: string,
}

interface WorkerResponse {
  data: Array<Worker>,
}

interface ParsedWorker {
  run_at: string,
  queue: string,
  payload: QueueJob
}

const useStyles = makeStyles((theme: Theme) =>
  createStyles({
    mainContent: {
      marginTop: theme.spacing(2),
    },
  }),
);

function fetchWorkers(): Promise<WorkerResponse> {
  return fetch('active_workers').then((response) => {
    if (!response.ok) {
      throw new Error(response.statusText);
    }
    return response.json()
  });
}

function parseWorkerPayload(payload: string): ParsedWorker {
  return JSON.parse(payload);
}

function payloadString(job: ParsedWorker): string {
  const date = new Date(job.run_at)
  return `Started: ${date.toLocaleString()} | Args: ${renderArguments(job.payload.args[0].arguments)}`;
}

function runningWorker(item: Worker) {
  const job = parseWorkerPayload(item.payload);
  return (
    <React.Fragment key={item.id}>
      <ListItem alignItems="flex-start">
        <ListItemText
          primary={`${item.id.split(':')[0]} - ${job.payload.args[0].job_class}`}
          secondary={payloadString(job)}
        />
      </ListItem>
      <Divider variant="fullWidth" component="li" />
    </React.Fragment>
  );
}

function idleWorker(item: Worker) {
  return (
    <React.Fragment key={item.id}>
      <ListItem alignItems="flex-start">
        <ListItemText
          primary={item.id.split(':')[0]}
          secondary="waiting..."
        />
      </ListItem>
      <Divider variant="fullWidth" component="li" />
    </React.Fragment>
  );
}

function renderListItem(item: Worker) {
  if (item.payload) {
    return runningWorker(item);
  }
  return idleWorker(item);
}

const Workers = () => {
  const classes = useStyles();
  const [workers, setWorkers] = useState<Array<Worker>>([]);
  useEffect(() => {
    fetchWorkers().then(body => setWorkers(body.data)).catch(err => console.error(err))
  }, [])
  const working = workers.filter(worker => !!worker.payload)
  return (
    <div className={classes.mainContent}>
      <Typography variant="h6">{working.length} of {workers.length} working</Typography>
      <List>
        {workers.map(item => renderListItem(item))}
      </List>
    </div>
  );
}

export default Workers;