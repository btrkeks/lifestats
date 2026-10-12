# LifeStats Systemd Service Files

This directory contains systemd service and timer files for automating LifeStats tasks.

## Fitbit Daily Sync

The `lifestats-fitbit-sync.service` and `lifestats-fitbit-sync.timer` automate daily synchronization of Fitbit data to your LifeStats database.

### Prerequisites

1. **LifeStats Web API must be running**: The service requires the LifeStats web API to be accessible at `http://localhost:3000`
2. **Fitbit OAuth configured**: You must have already authorized your Fitbit account through the web API's OAuth flow
3. **curl installed**: The service uses `curl` to make HTTP requests

### Installation

#### Option 1: System-wide installation (requires root)

```bash
# Copy the service files to the systemd directory
sudo cp systemd/lifestats-fitbit-sync.service /etc/systemd/system/
sudo cp systemd/lifestats-fitbit-sync.timer /etc/systemd/system/

# Reload systemd to recognize the new files
sudo systemctl daemon-reload

# Enable the timer to start on boot
sudo systemctl enable lifestats-fitbit-sync.timer

# Start the timer immediately
sudo systemctl start lifestats-fitbit-sync.timer
```

#### Option 2: User-level installation (recommended for single-user systems)

```bash
# Create user systemd directory if it doesn't exist
mkdir -p ~/.config/systemd/user/

# Copy the service files to the user systemd directory
cp systemd/lifestats-fitbit-sync.service ~/.config/systemd/user/
cp systemd/lifestats-fitbit-sync.timer ~/.config/systemd/user/

# Reload systemd to recognize the new files
systemctl --user daemon-reload

# Enable the timer to start on login
systemctl --user enable lifestats-fitbit-sync.timer

# Start the timer immediately
systemctl --user start lifestats-fitbit-sync.timer
```

### Configuration

#### Changing the Sync Time

By default, the sync runs at 8:00 AM daily. To change this:

1. Edit the timer file (`~/.config/systemd/user/lifestats-fitbit-sync.timer` or `/etc/systemd/system/lifestats-fitbit-sync.timer`)
2. Modify the `OnCalendar` line:
   ```ini
   # Example: Run at 6:30 AM instead
   OnCalendar=*-*-* 06:30:00
   ```
3. Reload and restart:
   ```bash
   # For user services
   systemctl --user daemon-reload
   systemctl --user restart lifestats-fitbit-sync.timer

   # For system services
   sudo systemctl daemon-reload
   sudo systemctl restart lifestats-fitbit-sync.timer
   ```

#### Running as a Specific User

If using system-wide installation, uncomment and set the `User` and `Group` directives in the service file:

```ini
[Service]
Type=oneshot
User=your-username
Group=your-group
# ... rest of configuration
```

#### Changing the API URL

If your web API runs on a different host or port, edit the service file and modify the `ExecStart` line:

```bash
# Example: API running on port 8080
ExecStart=/bin/bash -c 'YESTERDAY=$(date -d "yesterday" +%%Y-%%m-%%d); \
  TODAY=$(date +%%Y-%%m-%%d); \
  curl -X POST http://localhost:8080/integrations/fitbit/sync \
    -H "Content-Type: application/json" \
    -d "{\"since\": \"$YESTERDAY\", \"until\": \"$TODAY\"}" \
    --fail --silent --show-error'
```

### Usage

#### Check Timer Status

```bash
# For user services
systemctl --user status lifestats-fitbit-sync.timer

# For system services
sudo systemctl status lifestats-fitbit-sync.timer
```

#### View Timer Schedule

```bash
# For user services
systemctl --user list-timers lifestats-fitbit-sync.timer

# For system services
systemctl list-timers lifestats-fitbit-sync.timer
```

#### Manually Trigger Sync

```bash
# For user services
systemctl --user start lifestats-fitbit-sync.service

# For system services
sudo systemctl start lifestats-fitbit-sync.service
```

#### View Logs

```bash
# For user services
journalctl --user -u lifestats-fitbit-sync.service

# For system services
sudo journalctl -u lifestats-fitbit-sync.service

# Follow logs in real-time
journalctl --user -u lifestats-fitbit-sync.service -f
```

#### Stop/Disable Timer

```bash
# For user services
systemctl --user stop lifestats-fitbit-sync.timer
systemctl --user disable lifestats-fitbit-sync.timer

# For system services
sudo systemctl stop lifestats-fitbit-sync.timer
sudo systemctl disable lifestats-fitbit-sync.timer
```

### Troubleshooting

#### Service Fails to Start

1. **Check if the web API is running**:
   ```bash
   curl http://localhost:3000/statistics
   ```

2. **Verify Fitbit OAuth token exists**:
   - Ensure you've completed the OAuth flow via the web UI
   - Check the token storage location (typically in your database)

3. **View detailed error logs**:
   ```bash
   journalctl --user -u lifestats-fitbit-sync.service -n 50
   ```

#### API Connection Issues

- Ensure the API URL in the service file matches your running instance
- Check firewall rules if running remotely
- Verify network connectivity

#### Sync Not Running at Expected Time

- Check timer status: `systemctl --user list-timers lifestats-fitbit-sync.timer`
- Verify system time and timezone settings: `timedatectl status`
- Check if systemd timer service is running: `systemctl --user status systemd-timers.target`

### Advanced Configuration

#### Syncing Multiple Days

To sync the last 7 days instead of the default two-day window, modify the `ExecStart` line:

```bash
ExecStart=/bin/bash -c 'WEEK_AGO=$(date -d "7 days ago" +%%Y-%%m-%%d); \
  TODAY=$(date +%%Y-%%m-%%d); \
  curl -X POST http://localhost:3000/integrations/fitbit/sync \
    -H "Content-Type: application/json" \
    -d "{\"since\": \"$WEEK_AGO\", \"until\": \"$TODAY\"}" \
    --fail --silent --show-error'
```

**Note**: The Fitbit sync API has a maximum range of 31 days per request.

#### Adding Retry Logic

To add automatic retries on failure, uncomment the restart directives in the service file:

```ini
[Service]
Type=oneshot
Restart=on-failure
RestartSec=5m
# ... rest of configuration
```

This will retry the sync after 5 minutes if it fails.

### Security Considerations

- The service makes HTTP requests to localhost by default
- If exposing the API externally, consider:
  - Using HTTPS with proper TLS certificates
  - Adding authentication headers to the curl command
  - Using systemd's `EnvironmentFile` to store credentials securely

Example with authentication:
```ini
[Service]
Type=oneshot
EnvironmentFile=/etc/lifestats/api-credentials
ExecStart=/bin/bash -c 'YESTERDAY=$(date -d "yesterday" +%%Y-%%m-%%d); \
  TODAY=$(date +%%Y-%%m-%%d); \
  curl -X POST http://localhost:3000/integrations/fitbit/sync \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $API_TOKEN" \
    -d "{\"since\": \"$YESTERDAY\", \"until\": \"$TODAY\"}" \
    --fail --silent --show-error'
```
