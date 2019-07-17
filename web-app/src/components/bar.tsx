import React from 'react';
import { createStyles, makeStyles, Theme } from '@material-ui/core/styles';
import AppBar from '@material-ui/core/AppBar';
import Typography from '@material-ui/core/Typography';
import Toolbar from '@material-ui/core/Toolbar';

interface BarProps {
  failureCount: Number,
  successCount: Number,
}

const useStyles = makeStyles((theme: Theme) =>
  createStyles({
    root: {
      flexGrow: 1,
    },
    title: {
      flexGrow: 1,
    },
    stats: {
      marginRight: theme.spacing(2),
    },
  }),
);

const Bar: React.FC<BarProps> = (props) => {
  const classes = useStyles();
  return (
    <div className={classes.root}>
      <AppBar position="static">
        <Toolbar>
          <Typography variant="h6" className={classes.title}>
            Resque
        </Typography>
          <span className={classes.stats}>
            Total Failed: {props.failureCount}
          </span>
          <span>
            Total Succeeded: {props.successCount}
          </span>
        </Toolbar>
      </AppBar>
    </div>
  );
};

export default Bar;