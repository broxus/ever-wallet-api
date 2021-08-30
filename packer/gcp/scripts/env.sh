#!/bin/bash -eE

export DB_HOST=`curl --retry 5 -sfH "Metadata-Flavor: Google" "http://metadata/computeMetadata/v1/instance/attributes/db_host"`
export DB_NAME=`curl --retry 5 -sfH "Metadata-Flavor: Google" "http://metadata/computeMetadata/v1/instance/attributes/db_name"`
export DB_USER=`curl --retry 5 -sfH "Metadata-Flavor: Google" "http://metadata/computeMetadata/v1/instance/attributes/db_user"`
export DB_PASSWORD=`curl --retry 5 -sfH "Metadata-Flavor: Google" "http://metadata/computeMetadata/v1/instance/attributes/db_password"`

export API_SECRET=`curl --retry 5 -sfH "Metadata-Flavor: Google" "http://metadata/computeMetadata/v1/instance/attributes/api_secret"`

export DATABASE_URL="postgresql://$DB_USER:$DB_PASSWORD@$DB_HOST/$DB_NAME"
