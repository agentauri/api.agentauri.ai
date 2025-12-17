# Custom Grafana image with AgentAuri CloudWatch dashboards
FROM grafana/grafana:10.2.0

# Copy datasource provisioning
COPY provisioning/datasources/cloudwatch.yml /etc/grafana/provisioning/datasources/cloudwatch.yml

# Copy dashboard provisioning config
COPY provisioning/dashboards/default.yml /etc/grafana/provisioning/dashboards/default.yml

# Copy CloudWatch dashboards
COPY dashboards-cloudwatch/*.json /etc/grafana/dashboards/

# Set correct permissions
USER root
RUN chown -R grafana:root /etc/grafana/provisioning /etc/grafana/dashboards && \
    chmod -R 755 /etc/grafana/provisioning /etc/grafana/dashboards

USER grafana
