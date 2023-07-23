export RWATCH_HOME=$GIT_HOME/rwatch
alias rwatch-notes='cd $RWATCH_HOME && tree > $RWATCH_HOME/.notes && cat src/*.rs >> $RWATCH_HOME/.notes && cd - >> /dev/null'
