import React, { useEffect, useState } from 'react';
import { createStyles, makeStyles, Theme } from '@material-ui/core/styles';
import List from '@material-ui/core/List';
import ListItem from '@material-ui/core/ListItem';
import ListItemText from '@material-ui/core/ListItemText';
import ListItemSecondaryAction from '@material-ui/core/ListItemSecondaryAction';
import ListItemAvatar from '@material-ui/core/ListItemAvatar';
import Avatar from '@material-ui/core/Avatar';
import FavIcon from '@material-ui/icons/Favorite';
import WarnIcon from '@material-ui/icons/Warning';
import Divider from '@material-ui/core/Divider';
import { renderArguments, QueueJob } from '../utils/jobArgs';
import { Typography } from '@material-ui/core';
import WorkerDelete from './workerDelete';

interface Worker {
  id: string,
  payload?: Payload,
  heartbeat?: string,
}

interface WorkerResponse {
  data: Array<Worker>,
}

interface Payload {
  run_at: string,
  queue: string,
  payload: QueueJob
}

const HEARTBEAT_LIMIT = 30 * 60000

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

function payloadString(job: Payload): string {
  const date = new Date(job.run_at)
  return `Started: ${date.toLocaleString()} | Args: ${renderArguments(job.payload.args[0].arguments)}`;
}

function workerText(item: Worker) {
  if (!item.payload) {
    return (
      <ListItemText primary={item.id} secondary="waiting..." />
    );
  }
  return (
    <ListItemText 
      primary={`${item.id} - ${item.payload.payload.args[0].job_class}`}
      secondary={payloadString(item.payload)}
    />
  );
}

function heartbeatExpired(worker: Worker): boolean {
  if (!worker.heartbeat) {
    return true;
  }
  const then = new Date(worker.heartbeat);
  const now = Date.now();
  if ((now - then.valueOf()) > HEARTBEAT_LIMIT) {
    return true;
  }
  return false;
}

const Workers = () => {
  const classes = useStyles();
  const [workers, setWorkers] = useState<Array<Worker>>([]);
  useEffect(() => {
    fetchWorkers().then(body => setWorkers(body.data)).catch(err => console.error(err))
  }, [])
  const filterWorker = (workerId: string) => setWorkers(workers.filter(w => w.id !== workerId));
  const workerItem = (item: Worker) => {
    const expired = heartbeatExpired(item);
    return (
      <React.Fragment key={item.id}>
        <ListItem alignItems="flex-start">
          <ListItemAvatar>
            <Avatar>
              {expired ? <WarnIcon /> : <FavIcon />}
            </Avatar>
          </ListItemAvatar>
          {workerText(item)}
          {expired && (
            <ListItemSecondaryAction>
              <WorkerDelete workerId={item.id} onDelete={filterWorker} />
            </ListItemSecondaryAction>
          )}
        </ListItem>
        <Divider variant="fullWidth" component="li" />
      </React.Fragment>
    );
  };
  const working = workers.filter(worker => !!worker.payload)
  return (
    <div className={classes.mainContent}>
      <Typography variant="h6">{working.length} of {workers.length} working</Typography>
      <List>
        {workers.map(item => workerItem(item))}
      </List>
    </div>
  );
}

export default Workers;