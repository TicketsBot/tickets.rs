# export sharder id
export SHARDER_ID=$(python3 -c "import os; print(os.uname()[1].split('-')[1])")
exec /srv/sharder/sharder