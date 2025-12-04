# Production Database Setup Guide

Production-grade PostgreSQL deployment with TLS encryption, high availability, and automated backups.

## Table of Contents

1. [AWS RDS PostgreSQL](#aws-rds-postgresql)
2. [Azure Database for PostgreSQL](#azure-database-for-postgresql)
3. [Google Cloud SQL](#google-cloud-sql)
4. [Self-Managed PostgreSQL](#self-managed-postgresql)
5. [Connection Configuration](#connection-configuration)
6. [Monitoring & Alerts](#monitoring--alerts)

---

## AWS RDS PostgreSQL

### Prerequisites

- AWS CLI installed and configured
- AWS KMS key created for encryption
- VPC and security groups configured

### 1. Create Encrypted RDS Instance

```bash
#!/bin/bash
# Create production PostgreSQL instance with encryption at rest

DB_INSTANCE_ID="agentauri-prod"
DB_INSTANCE_CLASS="db.t3.medium"  # Adjust based on workload
DB_STORAGE_GB=100                  # Initial storage
KMS_KEY_ARN="arn:aws:kms:us-east-1:123456789012:key/12345678-1234-1234-1234-123456789012"
VPC_SECURITY_GROUP="sg-xxxxxxxxx"
DB_SUBNET_GROUP="agentauri-db-subnet-group"
MASTER_PASSWORD="$(openssl rand -base64 32)"  # Generate strong password

aws rds create-db-instance \
  --db-instance-identifier "$DB_INSTANCE_ID" \
  --db-instance-class "$DB_INSTANCE_CLASS" \
  --engine postgres \
  --engine-version 15.4 \
  --master-username postgres \
  --master-user-password "$MASTER_PASSWORD" \
  --allocated-storage "$DB_STORAGE_GB" \
  --storage-type gp3 \
  --storage-encrypted \
  --kms-key-id "$KMS_KEY_ARN" \
  --backup-retention-period 7 \
  --preferred-backup-window "03:00-04:00" \
  --preferred-maintenance-window "sun:04:00-sun:05:00" \
  --multi-az \
  --db-subnet-group-name "$DB_SUBNET_GROUP" \
  --vpc-security-group-ids "$VPC_SECURITY_GROUP" \
  --publicly-accessible false \
  --enable-iam-database-authentication \
  --enable-cloudwatch-logs-exports postgresql,upgrade \
  --deletion-protection \
  --auto-minor-version-upgrade \
  --tags Key=Project,Value=ERC-8004 Key=Environment,Value=Production

# Wait for instance to be available
aws rds wait db-instance-available --db-instance-identifier "$DB_INSTANCE_ID"

# Get endpoint
ENDPOINT=$(aws rds describe-db-instances \
  --db-instance-identifier "$DB_INSTANCE_ID" \
  --query 'DBInstances[0].Endpoint.Address' \
  --output text)

echo "Database created successfully!"
echo "Endpoint: $ENDPOINT"
echo "Master password: $MASTER_PASSWORD"
echo "⚠️  SAVE PASSWORD TO AWS SECRETS MANAGER IMMEDIATELY!"
```

### 2. Store Credentials in AWS Secrets Manager

```bash
# Store master password
aws secretsmanager create-secret \
  --name agentauri/prod/database/master-password \
  --description "ERC-8004 Production PostgreSQL master password" \
  --secret-string "$MASTER_PASSWORD" \
  --tags Key=Project,Value=ERC-8004 Key=Environment,Value=Production

# Store connection string
aws secretsmanager create-secret \
  --name agentauri/prod/database/connection-string \
  --description "ERC-8004 Production PostgreSQL connection string" \
  --secret-string "postgresql://postgres:$MASTER_PASSWORD@$ENDPOINT:5432/agentauri_backend?sslmode=verify-full&sslrootcert=/etc/ssl/certs/rds-ca-2019-root.pem" \
  --tags Key=Project,Value=ERC-8004 Key=Environment,Value=Production
```

### 3. Download RDS CA Certificate

```bash
# Download RDS root certificate bundle
wget https://truststore.pki.rds.amazonaws.com/global/global-bundle.pem \
  -O /etc/ssl/certs/rds-ca-bundle.pem

# Or download region-specific certificate
wget https://truststore.pki.rds.amazonaws.com/us-east-1/us-east-1-bundle.pem \
  -O /etc/ssl/certs/rds-ca-us-east-1.pem
```

### 4. Configure Application

```bash
# In production .env or environment variables
DATABASE_URL="postgresql://postgres:PASSWORD@agentauri-prod.xxxxxxxxx.us-east-1.rds.amazonaws.com:5432/agentauri_backend?sslmode=verify-full&sslrootcert=/etc/ssl/certs/rds-ca-bundle.pem"

# Or retrieve from Secrets Manager at runtime
# See: https://docs.aws.amazon.com/secretsmanager/latest/userguide/retrieving-secrets.html
```

### 5. Enable Enhanced Monitoring

```bash
# Create IAM role for enhanced monitoring
aws iam create-role \
  --role-name rds-monitoring-role \
  --assume-role-policy-document '{
    "Version": "2012-10-17",
    "Statement": [{
      "Sid": "",
      "Effect": "Allow",
      "Principal": {"Service": "monitoring.rds.amazonaws.com"},
      "Action": "sts:AssumeRole"
    }]
  }'

aws iam attach-role-policy \
  --role-name rds-monitoring-role \
  --policy-arn arn:aws:iam::aws:policy/service-role/AmazonRDSEnhancedMonitoringRole

# Enable enhanced monitoring on instance
aws rds modify-db-instance \
  --db-instance-identifier agentauri-prod \
  --monitoring-interval 60 \
  --monitoring-role-arn arn:aws:iam::123456789012:role/rds-monitoring-role
```

### 6. Create Read Replica (Optional - High Availability)

```bash
# Create read replica in different AZ
aws rds create-db-instance-read-replica \
  --db-instance-identifier agentauri-prod-replica-1 \
  --source-db-instance-identifier agentauri-prod \
  --db-instance-class db.t3.medium \
  --availability-zone us-east-1b \
  --storage-encrypted \
  --kms-key-id "$KMS_KEY_ARN" \
  --multi-az false \
  --tags Key=Project,Value=ERC-8004 Key=Environment,Value=Production Key=Role,Value=Replica
```

### Cost Estimate (AWS RDS)

| Component | Spec | Monthly Cost (USD) |
|-----------|------|-------------------|
| db.t3.medium (Multi-AZ) | 2 vCPU, 4GB RAM | $120 |
| Storage (100GB, gp3) | 100GB | $12 |
| Backup (100GB) | 7-day retention | $10 |
| Data Transfer | 100GB/month | $9 |
| **Total** | | **~$151/month** |

For production workload (db.m5.xlarge): ~$600-800/month

---

## Azure Database for PostgreSQL

### 1. Create Resource Group

```bash
az group create \
  --name agentauri-prod \
  --location eastus
```

### 2. Create PostgreSQL Server (Flexible Server)

```bash
# Generate strong password
ADMIN_PASSWORD="$(openssl rand -base64 32)"

# Create server with encryption (always enabled)
az postgres flexible-server create \
  --resource-group agentauri-prod \
  --name agentauri-postgres \
  --location eastus \
  --admin-user postgres \
  --admin-password "$ADMIN_PASSWORD" \
  --sku-name Standard_D2s_v3 \
  --tier GeneralPurpose \
  --storage-size 128 \
  --version 15 \
  --high-availability ZoneRedundant \
  --zone 1 \
  --standby-zone 2 \
  --backup-retention 7 \
  --public-access None \
  --tags Project=ERC-8004 Environment=Production

# Get connection info
az postgres flexible-server show \
  --resource-group agentauri-prod \
  --name agentauri-postgres \
  --query "{FQDN:fullyQualifiedDomainName, State:state}" \
  -o table

echo "Admin password: $ADMIN_PASSWORD"
echo "⚠️  SAVE PASSWORD TO AZURE KEY VAULT IMMEDIATELY!"
```

### 3. Store Credentials in Azure Key Vault

```bash
# Create Key Vault
az keyvault create \
  --name agentauri-prod-kv \
  --resource-group agentauri-prod \
  --location eastus

# Store password
az keyvault secret set \
  --vault-name agentauri-prod-kv \
  --name database-password \
  --value "$ADMIN_PASSWORD"

# Store connection string
az keyvault secret set \
  --vault-name agentauri-prod-kv \
  --name database-connection-string \
  --value "postgresql://postgres:$ADMIN_PASSWORD@agentauri-postgres.postgres.database.azure.com:5432/agentauri_backend?sslmode=verify-full"
```

### 4. Download Azure CA Certificate

```bash
# Download BaltimoreCyberTrustRoot certificate
wget https://www.digicert.com/CACerts/BaltimoreCyberTrustRoot.crt.pem \
  -O /etc/ssl/certs/azure-postgresql-ca.pem
```

### 5. Configure Firewall (Allow Application IPs)

```bash
# Get application's public IP
APP_PUBLIC_IP=$(curl -s ifconfig.me)

# Allow application IP
az postgres flexible-server firewall-rule create \
  --resource-group agentauri-prod \
  --name agentauri-postgres \
  --rule-name allow-app-server \
  --start-ip-address "$APP_PUBLIC_IP" \
  --end-ip-address "$APP_PUBLIC_IP"
```

### 6. Enable Customer-Managed Encryption Key (Optional)

```bash
# Create encryption key in Key Vault
az keyvault key create \
  --vault-name agentauri-prod-kv \
  --name database-encryption-key \
  --kty RSA \
  --size 2048

# Enable customer-managed key encryption
az postgres flexible-server update \
  --resource-group agentauri-prod \
  --name agentauri-postgres \
  --key-id "https://agentauri-prod-kv.vault.azure.net/keys/database-encryption-key"
```

### Cost Estimate (Azure)

| Component | Spec | Monthly Cost (USD) |
|-----------|------|-------------------|
| Standard_D2s_v3 (Zone Redundant HA) | 2 vCPU, 8GB RAM | $240 |
| Storage (128GB) | 128GB | $16 |
| Backup (128GB) | 7-day retention | $13 |
| **Total** | | **~$269/month** |

---

## Google Cloud SQL

### 1. Create Cloud SQL Instance

```bash
# Generate strong password
ADMIN_PASSWORD="$(openssl rand -base64 32)"

# Create instance with encryption (always enabled)
gcloud sql instances create agentauri-postgres \
  --database-version=POSTGRES_15 \
  --tier=db-custom-2-7680 \
  --region=us-central1 \
  --storage-type=SSD \
  --storage-size=100GB \
  --storage-auto-increase \
  --backup \
  --backup-start-time=03:00 \
  --maintenance-window-day=SUN \
  --maintenance-window-hour=4 \
  --availability-type=REGIONAL \
  --no-assign-ip \
  --network=projects/PROJECT_ID/global/networks/default \
  --database-flags=cloudsql.enable_pgaudit=on \
  --root-password="$ADMIN_PASSWORD"

# Create database
gcloud sql databases create agentauri_backend \
  --instance=agentauri-postgres

echo "Database created successfully!"
echo "Admin password: $ADMIN_PASSWORD"
echo "⚠️  SAVE PASSWORD TO SECRET MANAGER IMMEDIATELY!"
```

### 2. Store Credentials in Secret Manager

```bash
# Store password
echo -n "$ADMIN_PASSWORD" | gcloud secrets create database-password \
  --data-file=- \
  --replication-policy=automatic

# Store connection string
gcloud secrets create database-connection-string \
  --data-file=- <<EOF
postgresql://postgres:$ADMIN_PASSWORD@/agentauri_backend?host=/cloudsql/PROJECT_ID:us-central1:agentauri-postgres&sslmode=disable
EOF
```

### 3. Use Cloud SQL Proxy (Recommended)

```bash
# Download Cloud SQL Proxy
wget https://dl.google.com/cloudsql/cloud_sql_proxy.linux.amd64 -O cloud_sql_proxy
chmod +x cloud_sql_proxy

# Run proxy (creates Unix socket)
./cloud_sql_proxy -instances=PROJECT_ID:us-central1:agentauri-postgres=tcp:5432 &

# Connection string (via proxy - TLS handled automatically)
DATABASE_URL="postgresql://postgres:$ADMIN_PASSWORD@localhost:5432/agentauri_backend"
```

### 4. Enable Customer-Managed Encryption Key (Optional)

```bash
# Create encryption key
gcloud kms keyrings create agentauri-keyring \
  --location=us-central1

gcloud kms keys create database-encryption-key \
  --location=us-central1 \
  --keyring=agentauri-keyring \
  --purpose=encryption

# Update instance to use customer-managed key
gcloud sql instances patch agentauri-postgres \
  --disk-encryption-key=projects/PROJECT_ID/locations/us-central1/keyRings/agentauri-keyring/cryptoKeys/database-encryption-key
```

### Cost Estimate (GCP)

| Component | Spec | Monthly Cost (USD) |
|-----------|------|-------------------|
| db-custom-2-7680 (Regional HA) | 2 vCPU, 7.5GB RAM | $200 |
| Storage (100GB SSD) | 100GB | $17 |
| Backup (100GB) | 7-day retention | $8 |
| **Total** | | **~$225/month** |

---

## Self-Managed PostgreSQL

For maximum control, deploy PostgreSQL on your own infrastructure.

### 1. Install PostgreSQL 15 with TimescaleDB

**Ubuntu 22.04**:
```bash
# Add PostgreSQL repository
wget --quiet -O - https://www.postgresql.org/media/keys/ACCC4CF8.asc | sudo apt-key add -
echo "deb http://apt.postgresql.org/pub/repos/apt $(lsb_release -cs)-pgdg main" | sudo tee /etc/apt/sources.list.d/pgdg.list

# Install PostgreSQL 15
sudo apt update
sudo apt install -y postgresql-15 postgresql-contrib-15

# Add TimescaleDB repository
echo "deb https://packagecloud.io/timescale/timescaledb/ubuntu/ $(lsb_release -c -s) main" | sudo tee /etc/apt/sources.list.d/timescaledb.list
wget --quiet -O - https://packagecloud.io/timescale/timescaledb/gpgkey | sudo apt-key add -

# Install TimescaleDB
sudo apt update
sudo apt install -y timescaledb-2-postgresql-15

# Tune PostgreSQL for TimescaleDB
sudo timescaledb-tune --quiet --yes

# Restart PostgreSQL
sudo systemctl restart postgresql
```

### 2. Generate Production TLS Certificates (Let's Encrypt)

```bash
# Install Certbot
sudo apt install -y certbot

# Generate certificate (requires DNS setup)
sudo certbot certonly --standalone \
  -d db.yourdomain.com \
  --email admin@yourdomain.com \
  --agree-tos \
  --non-interactive

# Copy certificates to PostgreSQL data directory
sudo cp /etc/letsencrypt/live/db.yourdomain.com/fullchain.pem /var/lib/postgresql/15/main/server.crt
sudo cp /etc/letsencrypt/live/db.yourdomain.com/privkey.pem /var/lib/postgresql/15/main/server.key
sudo chown postgres:postgres /var/lib/postgresql/15/main/server.{crt,key}
sudo chmod 600 /var/lib/postgresql/15/main/server.key

# Setup auto-renewal (runs twice daily)
echo "0 0,12 * * * root certbot renew --quiet && systemctl reload postgresql" | sudo tee -a /etc/crontab
```

### 3. Configure PostgreSQL for Production

Copy our production config:
```bash
sudo cp docker/postgres/postgresql.conf /etc/postgresql/15/main/postgresql.conf
sudo cp docker/postgres/pg_hba.conf /etc/postgresql/15/main/pg_hba.conf

# Edit postgresql.conf to use correct certificate paths
sudo sed -i "s|/var/lib/postgresql/|/var/lib/postgresql/15/main/|g" /etc/postgresql/15/main/postgresql.conf

# Restart PostgreSQL
sudo systemctl restart postgresql
```

### 4. Enable Filesystem Encryption (LUKS)

```bash
# Install cryptsetup
sudo apt install -y cryptsetup

# Create encrypted volume (WARNING: Destroys data on /dev/sdb!)
sudo cryptsetup luksFormat /dev/sdb

# Open encrypted volume
sudo cryptsetup open /dev/sdb pgdata

# Format and mount
sudo mkfs.ext4 /dev/mapper/pgdata
sudo mkdir -p /mnt/pgdata
sudo mount /dev/mapper/pgdata /mnt/pgdata

# Move PostgreSQL data
sudo systemctl stop postgresql
sudo rsync -av /var/lib/postgresql/15/main/ /mnt/pgdata/
sudo chown -R postgres:postgres /mnt/pgdata

# Update data directory in config
sudo sed -i "s|data_directory = '/var/lib/postgresql/15/main'|data_directory = '/mnt/pgdata'|" /etc/postgresql/15/main/postgresql.conf

# Setup auto-mount with key file
sudo dd if=/dev/urandom of=/root/pgdata.key bs=1024 count=4
sudo chmod 600 /root/pgdata.key
sudo cryptsetup luksAddKey /dev/sdb /root/pgdata.key

echo "pgdata /dev/sdb /root/pgdata.key luks" | sudo tee -a /etc/crypttab
echo "/dev/mapper/pgdata /mnt/pgdata ext4 defaults 0 2" | sudo tee -a /etc/fstab

# Restart PostgreSQL
sudo systemctl start postgresql
```

### 5. Setup Automated Backups (pgBackRest)

```bash
# Install pgBackRest
sudo apt install -y pgbackrest

# Create backup directory
sudo mkdir -p /var/lib/pgbackrest
sudo chown postgres:postgres /var/lib/pgbackrest

# Configure pgBackRest
sudo tee /etc/pgbackrest.conf <<EOF
[global]
repo1-path=/var/lib/pgbackrest
repo1-retention-full=7
repo1-cipher-type=aes-256-cbc
repo1-cipher-pass=$(openssl rand -base64 32)

[agentauri]
pg1-path=/mnt/pgdata
pg1-port=5432
EOF

# Create initial backup
sudo -u postgres pgbackrest --stanza=agentauri stanza-create
sudo -u postgres pgbackrest --stanza=agentauri --type=full backup

# Schedule daily backups
echo "0 2 * * * postgres pgbackrest --stanza=agentauri --type=incr backup" | sudo tee -a /etc/crontab
```

### Cost Estimate (Self-Managed on AWS EC2)

| Component | Spec | Monthly Cost (USD) |
|-----------|------|-------------------|
| c5.xlarge instance | 4 vCPU, 8GB RAM | $125 |
| EBS Storage (gp3, 200GB) | 200GB | $20 |
| EBS Snapshots (200GB) | 7-day retention | $10 |
| Data Transfer | 100GB/month | $9 |
| **Total** | | **~$164/month** |

**Savings**: ~10% cheaper than managed, but requires operational expertise.

---

## Connection Configuration

### Rust Application (SQLx)

```rust
use sqlx::postgres::{PgConnectOptions, PgSslMode};
use sqlx::PgPool;

// Production connection with full TLS verification
let options = PgConnectOptions::new()
    .host("agentauri-prod.xxxxxxxxx.us-east-1.rds.amazonaws.com")
    .port(5432)
    .database("agentauri_backend")
    .username("postgres")
    .password(&secrets.database_password)  // From secrets manager
    .ssl_mode(PgSslMode::VerifyFull)       // Verify certificate and hostname
    .ssl_root_cert("/etc/ssl/certs/rds-ca-bundle.pem");

let pool = PgPool::connect_with(options).await?;
```

### Environment Variables

```bash
# AWS RDS
DATABASE_URL="postgresql://postgres:PASSWORD@agentauri-prod.xxxxx.us-east-1.rds.amazonaws.com:5432/agentauri_backend?sslmode=verify-full&sslrootcert=/etc/ssl/certs/rds-ca-bundle.pem"

# Azure
DATABASE_URL="postgresql://postgres:PASSWORD@agentauri-postgres.postgres.database.azure.com:5432/agentauri_backend?sslmode=verify-full&sslrootcert=/etc/ssl/certs/azure-postgresql-ca.pem"

# GCP (via Cloud SQL Proxy)
DATABASE_URL="postgresql://postgres:PASSWORD@localhost:5432/agentauri_backend"

# Self-Managed
DATABASE_URL="postgresql://postgres:PASSWORD@db.yourdomain.com:5432/agentauri_backend?sslmode=verify-full"
```

---

## Monitoring & Alerts

### CloudWatch Alarms (AWS)

```bash
# CPU utilization
aws cloudwatch put-metric-alarm \
  --alarm-name agentauri-db-high-cpu \
  --alarm-description "Database CPU >80%" \
  --metric-name CPUUtilization \
  --namespace AWS/RDS \
  --statistic Average \
  --period 300 \
  --threshold 80 \
  --comparison-operator GreaterThanThreshold \
  --dimensions Name=DBInstanceIdentifier,Value=agentauri-prod \
  --evaluation-periods 2 \
  --alarm-actions arn:aws:sns:us-east-1:123456789012:agentauri-alerts

# Storage space
aws cloudwatch put-metric-alarm \
  --alarm-name agentauri-db-low-storage \
  --alarm-description "Database storage <10GB free" \
  --metric-name FreeStorageSpace \
  --namespace AWS/RDS \
  --statistic Average \
  --period 300 \
  --threshold 10737418240 \  # 10GB in bytes
  --comparison-operator LessThanThreshold \
  --dimensions Name=DBInstanceIdentifier,Value=agentauri-prod \
  --evaluation-periods 1 \
  --alarm-actions arn:aws:sns:us-east-1:123456789012:agentauri-alerts

# Connection count
aws cloudwatch put-metric-alarm \
  --alarm-name agentauri-db-high-connections \
  --alarm-description "Database connections >80" \
  --metric-name DatabaseConnections \
  --namespace AWS/RDS \
  --statistic Average \
  --period 300 \
  --threshold 80 \
  --comparison-operator GreaterThanThreshold \
  --dimensions Name=DBInstanceIdentifier,Value=agentauri-prod \
  --evaluation-periods 2 \
  --alarm-actions arn:aws:sns:us-east-1:123456789012:agentauri-alerts
```

### Prometheus Metrics (Self-Managed)

```bash
# Install postgres_exporter
wget https://github.com/prometheus-community/postgres_exporter/releases/download/v0.15.0/postgres_exporter-0.15.0.linux-amd64.tar.gz
tar xvzf postgres_exporter-0.15.0.linux-amd64.tar.gz
sudo mv postgres_exporter-0.15.0.linux-amd64/postgres_exporter /usr/local/bin/

# Create systemd service
sudo tee /etc/systemd/system/postgres_exporter.service <<EOF
[Unit]
Description=PostgreSQL Exporter
After=network.target

[Service]
Type=simple
User=postgres
Environment=DATA_SOURCE_NAME=postgresql://postgres:PASSWORD@localhost:5432/postgres?sslmode=require
ExecStart=/usr/local/bin/postgres_exporter
Restart=always

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl enable --now postgres_exporter
```

---

## Security Checklist

Production database MUST have:

- [ ] **Encryption at rest** (managed database or LUKS)
- [ ] **Encryption in transit** (TLS 1.2+ with verify-full)
- [ ] **Strong passwords** (32+ characters, random)
- [ ] **Secrets manager** (AWS/Azure/GCP, not environment variables)
- [ ] **Firewall rules** (allow only application IPs)
- [ ] **Private network** (no public IP)
- [ ] **Automated backups** (7+ day retention)
- [ ] **Point-in-time recovery** (PITR enabled)
- [ ] **High availability** (Multi-AZ or zone-redundant)
- [ ] **Monitoring** (CPU, memory, connections, storage)
- [ ] **Alerts** (critical metrics, email/Slack/PagerDuty)
- [ ] **Audit logging** (CloudWatch/Azure Monitor/Cloud Logging)
- [ ] **Access controls** (IAM authentication where possible)
- [ ] **Certificate validation** (verify-full, not require)
- [ ] **Regular updates** (auto minor version upgrades)

---

## Next Steps

1. Choose your platform (AWS/Azure/GCP/Self-Managed)
2. Follow setup guide above
3. Test TLS connection: `./scripts/test-pg-tls.sh HOSTNAME 5432 agentauri_backend postgres`
4. Apply database migrations: `sqlx migrate run`
5. Enable column encryption for PII (see [DATABASE_ENCRYPTION.md](./DATABASE_ENCRYPTION.md))
6. Setup monitoring and alerts
7. Test disaster recovery procedures
8. Document runbook for team

## Support

- AWS RDS: https://docs.aws.amazon.com/rds/
- Azure PostgreSQL: https://docs.microsoft.com/azure/postgresql/
- GCP Cloud SQL: https://cloud.google.com/sql/docs/
- PostgreSQL Docs: https://www.postgresql.org/docs/15/
