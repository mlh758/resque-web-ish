import React from 'react';
import { createStyles, makeStyles, Theme } from '@material-ui/core/styles';
import ExpansionPanel from '@material-ui/core/ExpansionPanel';
import ExpansionPanelSummary from '@material-ui/core/ExpansionPanelSummary';
import ExpansionPanelDetails from '@material-ui/core/ExpansionPanelDetails';
import ExpansionPanelActions from '@material-ui/core/ExpansionPanelActions';
import Typography from '@material-ui/core/Typography';
import Grid from '@material-ui/core/Grid';
import DeleteIcon from '@material-ui/icons/Delete';
import RefreshIcon from '@material-ui/icons/Refresh';
import IconButton from '@material-ui/core/IconButton';
import ExpandMoreIcon from '@material-ui/icons/ExpandMore';
import { QueueJob, renderArguments } from '../utils/jobArgs';

export interface FailedJob {
  backtrace: Array<string>,
  error: string,
  exception: string,
  failed_at: string,
  payload: QueueJob,
  queue: string,
}

interface FailedProps {
  job: FailedJob,
  onJobDeleted(id: string): void,
  onJobRetried(id: string): void,
}

const useStyles = makeStyles((theme: Theme) =>
  createStyles({
    root: {
      width: '100%',
    },
    traceSpacing: {
      marginTop: theme.spacing(1),
    },
    gridRoot: {
      flexGrow: 1,
    },
    panelContents: {
      display: 'flex',
      flexFlow: 'row wrap',
      justifyContent: "space-between",
      flexBasis: '100%',
      alignItems: 'stretch',
    }
  }),
);

function trimmedError(err: string): string {
  const spacedErr = err.replace(/(,)\S/g, '$1 ');
  if (spacedErr.length > 1000) {
    return spacedErr.slice(0, 997) + '...';
  }
  return spacedErr;
}

interface RequestBody {
  method: string,
  headers: Record<string, string>,
  body: string,
}

function request(jobID: string, method: string): RequestBody {
  return {
    method: method,
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ id: jobID }),
  }
}

const FailedRow: React.FC<FailedProps> = ({ job, onJobDeleted, onJobRetried }) => {
  const classes = useStyles()
  const [deleting, setDeleting] = React.useState(false);
  const [retrying, setRetrying] = React.useState(false);
  const handleDeleteJob = () => {
    const jobID = job.payload.args[0].job_id;
    setDeleting(true);
    fetch("failed_job", request(jobID, 'DELETE')).then((response) => {
      if (response.ok) {
        onJobDeleted(jobID);
      } else {
        console.error(`unable to delete job ${jobID}`)
        setDeleting(false);
      }
    });
  }
  const handleRetryJob = () => {
    const jobID = job.payload.args[0].job_id;
    setRetrying(true);
    fetch("retry_job", request(jobID, 'POST')).then((response) => {
      if (response.ok) {
        onJobRetried(jobID);
      } else {
        console.error(`unable to retry job ${jobID}`)
        setRetrying(false);
      }
    });
  }
  return (
    <div className={classes.root}>
      <ExpansionPanel>
        <ExpansionPanelSummary expandIcon={<ExpandMoreIcon />}>
          <Grid className={classes.gridRoot} container>
            <Grid item xs={4}>
              <Typography variant="subtitle1">{job.payload.args[0].job_class}</Typography>
              <Typography variant="subtitle2" color="textSecondary">{job.exception}</Typography>
            </Grid>
            <Grid item xs={8}>
              <Typography variant="body2" color="secondary">{trimmedError(job.error)}</Typography>
            </Grid>
          </Grid>
        </ExpansionPanelSummary>
        <ExpansionPanelDetails>
          <div className={classes.panelContents}>
            <div>
              {job.backtrace.slice(0, Math.min(job.backtrace.length, 10)).map((traceLine, idx) => {
                return (<Typography variant="body2" className={classes.traceSpacing} key={idx}>{traceLine}</Typography>)
              })}
            </div>
            <div>
              <Typography variant="overline">
                Arguments: {renderArguments(job.payload.args[0].arguments)}
              </Typography>
            </div>
          </div>
        </ExpansionPanelDetails>
        <ExpansionPanelActions>
          <IconButton color="secondary" onClick={handleDeleteJob} disabled={deleting}>
            <DeleteIcon />
          </IconButton>
          <IconButton onClick={handleRetryJob} disabled={retrying}>
            <RefreshIcon />
          </IconButton>
        </ExpansionPanelActions>
      </ExpansionPanel>
    </div>
  );
};

export default FailedRow;