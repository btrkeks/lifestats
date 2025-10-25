-- Configuration
local options = {
    api_endpoint = "http://localhost:3000/sessions",
    statistic_id = 24,
    heartbeat_interval = 1,  -- in seconds
    target_directory = "/home/daniel/main/日本語/聞き取り練習"
}

-- Required module for making HTTP requests
local utils = require 'mp.utils'
local msg = require 'mp.msg'

-- Timer for periodic heartbeats
local heartbeat_timer = nil

-- Track session state
local session_active = false

-- Function to check if the current file is in the target directory
local function is_target_file()
    local path = mp.get_property("path")
    if not path then return false end

    -- Convert both paths to absolute paths for comparison
    local abs_path = utils.join_path(utils.getcwd(), path)
    return string.match(abs_path, "^" .. options.target_directory)
end

-- Helper to execute curl request and return HTTP status code
local function http_request(method, endpoint, body)
    local curl_cmd = {
        "curl",
        "-s",  -- silent mode
        "-X", method,
        "-H", "Content-Type: application/json",
        "-w", "%{http_code}",  -- capture HTTP status code
        endpoint
    }

    -- Add body if provided
    if body then
        table.insert(curl_cmd, "-d")
        table.insert(curl_cmd, body)
    end

    local process_info = utils.subprocess({
        args = curl_cmd,
        playback_only = false,
        capture_stdout = true,
        capture_stderr = true,
    })

    -- Extract HTTP status code from stdout (last 3 chars)
    local output = process_info.stdout or ""
    local status_code = tonumber(output:sub(-3))

    return process_info.status == 0, status_code
end

-- Start a new heartbeat session
local function start_session()
    if session_active then return end

    local endpoint = options.api_endpoint .. "/" .. options.statistic_id .. "/start"
    local success, http_code = http_request("POST", endpoint, '{"idle_timeout_seconds": 3600, "flush_interval_seconds": 60}')

    if success and http_code == 201 then
        session_active = true
        msg.debug("✅ Session started (HTTP " .. http_code .. ")")
    else
        msg.warn("❌ Failed to start session (HTTP " .. (http_code or "?") .. ")")
    end
end

-- Send a heartbeat to keep session alive
local function send_heartbeat()
    if not session_active or not is_target_file() then return end

    local endpoint = options.api_endpoint .. "/" .. options.statistic_id .. "/beat"
    local success, http_code = http_request("POST", endpoint)

    if success and http_code == 204 then
        msg.debug("💓 Heartbeat sent")
    else
        msg.warn("❌ Failed to send heartbeat (HTTP " .. (http_code or "?") .. ")")
        session_active = false
    end
end

-- Stop the heartbeat session
local function stop_session()
    if not session_active then return end

    local endpoint = options.api_endpoint .. "/" .. options.statistic_id .. "/stop"
    local success, http_code = http_request("POST", endpoint)

    if success and http_code == 204 then
        session_active = false
        msg.debug("⏹️  Session stopped (HTTP " .. http_code .. ")")
    else
        msg.warn("❌ Failed to stop session (HTTP " .. (http_code or "?") .. ")")
        session_active = false
    end
end

-- Function to start the heartbeat timer
local function start_heartbeat_timer()
    if heartbeat_timer then
        heartbeat_timer:kill()
    end
    heartbeat_timer = mp.add_periodic_timer(options.heartbeat_interval, send_heartbeat)
end

-- Function to stop the heartbeat timer
local function stop_heartbeat_timer()
    if heartbeat_timer then
        heartbeat_timer:kill()
        heartbeat_timer = nil
    end
end

local function on_video_start()
    local filename = mp.get_property("filename")
    msg.info("▶️  logging started: " .. (filename or "unknown"))
    start_session()
    start_heartbeat_timer()
end

local function on_video_end()
    msg.info("⏹️  logging stopped")
    mp.osd_message("Stopped logging", 1)
    stop_heartbeat_timer()
    stop_session()
end

-- Register event handlers
mp.observe_property("pause", "bool", function(name, value)
    if value then
	on_video_end()
    else
        if is_target_file() then
	    on_video_start()
        end
    end
end)

mp.register_event("file-loaded", function()
    local path = mp.get_property("path") or "unknown"
    if is_target_file() then
	on_video_start()
    else
        msg.debug("⏭️  skipped (not target): " .. path)
	on_video_end()
    end
end)

mp.register_event("end-file", function()
    on_video_end()
end)

-- Log plugin initialization
msg.info("✨ Lifestats logger initialized (monitoring: " .. options.target_directory .. ")")
