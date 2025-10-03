-- Configuration
local options = {
    endpoint = "http://localhost:8000/api/heartbeatlogger/24",
    update_interval = 1,  -- in seconds
    target_directory = "/home/daniel/main/日本語/聞き取り練習"
}

-- Required module for making HTTP requests
local utils = require 'mp.utils'
local msg = require 'mp.msg'

-- Timer for periodic updates
local update_timer = nil

-- Function to check if the current file is in the target directory
local function is_target_file()
    local path = mp.get_property("path")
    if not path then return false end
    
    -- Convert both paths to absolute paths for comparison
    local abs_path = utils.join_path(utils.getcwd(), path)
    return string.match(abs_path, "^" .. options.target_directory)
end

local function ping_logger()
    -- Only send request if we're playing a file from the target directory
    if not is_target_file() then return end
    
    -- Prepare the curl command
    local curl_cmd = {
        "curl",
        "-X", "POST",
        "-H", "Content-Type: application/json",
        options.endpoint .. "/ping"
    }
    
    -- Execute the curl command asynchronously
    local process_info = utils.subprocess({
        args = curl_cmd,
        playback_only = false,
        capture_stdout = true,
        capture_stderr = true,
    })
end

local function end_logger()
    -- Only send request if we're playing a file from the target directory
    if not is_target_file() then return end
    
    -- Prepare the curl command
    local curl_cmd = {
        "curl",
        "-X", "POST",
        "-H", "Content-Type: application/json",
        options.endpoint .. "/stop"
    }
    
    -- Execute the curl command asynchronously
    local process_info = utils.subprocess({
        args = curl_cmd,
        playback_only = false,
        capture_stdout = true,
        capture_stderr = true,
    })

    mp.osd_message("Stopped logging", 1)
end

-- Function to start the update timer
local function start_timer()
    if update_timer then
        update_timer:kill()
    end
    update_timer = mp.add_periodic_timer(options.update_interval, ping_logger)
end

-- Function to stop the update timer
local function stop_timer()
    if update_timer then
        update_timer:kill()
        update_timer = nil
    end
end

local function on_video_start()
	ping_logger()
	start_timer()
end

local function on_video_end()
	end_logger()
        stop_timer()
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
    if is_target_file() then
	on_video_start()
    else
	on_video_end()
    end
end)

mp.register_event("end-file", function()
    on_video_end()
end)

-- Log plugin initialization
msg.info("Lifestats logger initialized")
