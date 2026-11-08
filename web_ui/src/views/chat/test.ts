export const stream =
  [
    {
      "bind_event_id": 114,
      "type": "start",
      "event": "section",
      "data": {
        "reaction_id": 114,
        "response_id": 115
      }
    },
    {
      "bind_event_id": 114,
      "type": "open",
      "event": "section",
      "data": "think"
    },
    {
      "bind_event_id": 114,
      "type": "chunk",
      "event": "section",
      "data": "\r\n用"
    },
    {
      "bind_event_id": 114,
      "type": "chunk",
      "event": "section",
      "data": "户想了解北"
    },
    {
      "bind_event_id": 114,
      "type": "chunk",
      "event": "section",
      "data": "京的天气，"
    },
    {
      "bind_event_id": 114,
      "type": "chunk",
      "event": "section",
      "data": "我需要"
    },
    {
      "bind_event_id": 114,
      "type": "chunk",
      "event": "section",
      "data": "调"
    },
    {
      "bind_event_id": 114,
      "type": "chunk",
      "event": "section",
      "data": "用"
    },
    {
      "bind_event_id": 114,
      "type": "chunk",
      "event": "section",
      "data": "天"
    },
    {
      "bind_event_id": 114,
      "type": "chunk",
      "event": "section",
      "data": "气"
    },
    {
      "bind_event_id": 114,
      "type": "chunk",
      "event": "section",
      "data": "查"
    },
    {
      "bind_event_id": 114,
      "type": "chunk",
      "event": "section",
      "data": "询工"
    },
    {
      "bind_event_id": 114,
      "type": "chunk",
      "event": "section",
      "data": "具... \r\n"
    },
    {
      "bind_event_id": 114,
      "type": "close",
      "event": "section",
      "data": {
        "content": "\r\n用户想了解北京的天气，我需要调用天气查询工具... \r\n",
        "id": 118,
        "tag_name": "think",
        "time": "1ms 39us 700ns"
      }
    },
    {
      "bind_event_id": 114,
      "type": "open",
      "event": "section",
      "data": "tool_call"
    },
    {
      "bind_event_id": 114,
      "type": "close",
      "event": "section",
      "data": {
        "content": "http.request(method=\"get\",url=\"https://crates.io/crates/nanoid\")",
        "id": 119,
        "tag_name": "tool_call",
        "time": "1ms 224us 300ns"
      }
    },
    {
      "bind_event_id": 114,
      "type": "open",
      "event": "section",
      "data": "tool_call"
    },
    {
      "bind_event_id": 114,
      "type": "close",
      "event": "section",
      "data": {
        "content": "http.request(method=\"get\",url=\"https://crates.io111111123/crates/nanoid\")",
        "id": 120,
        "tag_name": "tool_call",
        "time": "1ms 817us 500ns"
      }
    },
    {
      "bind_event_id": 115,
      "type": "end",
      "event": "section",
      "data": {
        "reaction_id": 114,
        "response_id": 115
      }
    },
    {
      "bind_event_id": 116,
      "type": "start",
      "event": "tool_call",
      "data": [
        {
          "id": 119,
          "package": "http",
          "tool": "request"
        },
        {
          "id": 120,
          "package": "http",
          "tool": "request"
        }
      ]
    },
    {
      "bind_event_id": 116,
      "type": "active",
      "event": "tool_call",
      "data": 119
    },
    {
      "bind_event_id": 116,
      "type": "active",
      "event": "tool_call",
      "data": 120
    },
    {
      "bind_event_id": 116,
      "type": "success",
      "event": "tool_call",
      "data": {
        "content": "\"requestId: 6d4392d632d49770,status:404\"",
        "id": 119,
        "status": "success",
        "time": {
          "nanos": 884220200,
          "secs": 0
        }
      }
    },
    {
      "bind_event_id": 116,
      "type": "end",
      "event": "tool_call",
      "data": null
    },
    {
      "bind_event_id": 116,
      "type": "failure",
      "event": "tool_call",
      "data": {
        "content": "error sending request for url (https://crates.io111111123/crates/nanoid)",
        "id": 120,
        "status": "failure",
        "time": {
          "nanos": 157356600,
          "secs": 2
        }
      }
    },
    {
      "bind_event_id": 117,
      "type": "start",
      "event": "section",
      "data": {
        "reaction_id": 117,
        "response_id": 118
      }
    },
    {
      "bind_event_id": 117,
      "type": "open",
      "event": "section",
      "data": "think"
    },
    {
      "bind_event_id": 117,
      "type": "chunk",
      "event": "section",
      "data": "\r\n"
    },
    {
      "bind_event_id": 117,
      "type": "chunk",
      "event": "section",
      "data": "已"
    },
    {
      "bind_event_id": 117,
      "type": "chunk",
      "event": "section",
      "data": "获"
    },
    {
      "bind_event_id": 117,
      "type": "chunk",
      "event": "section",
      "data": "得"
    },
    {
      "bind_event_id": 117,
      "type": "chunk",
      "event": "section",
      "data": "查"
    },
    {
      "bind_event_id": 117,
      "type": "chunk",
      "event": "section",
      "data": "询结果\r\n"
    },
    {
      "bind_event_id": 117,
      "type": "close",
      "event": "section",
      "data": {
        "content": "\r\n已获得查询结果\r\n",
        "id": 121,
        "tag_name": "think",
        "time": "679us"
      }
    },
    {
      "bind_event_id": 118,
      "type": "open",
      "event": "section",
      "data": "reply"
    },
    {
      "bind_event_id": 118,
      "type": "chunk",
      "event": "section",
      "data": "\r\n根"
    },
    {
      "bind_event_id": 118,
      "type": "chunk",
      "event": "section",
      "data": "据查询结果"
    },
    {
      "bind_event_id": 118,
      "type": "chunk",
      "event": "section",
      "data": "，北京现在"
    },
    {
      "bind_event_id": 118,
      "type": "chunk",
      "event": "section",
      "data": "的天气是"
    },
    {
      "bind_event_id": 118,
      "type": "chunk",
      "event": "section",
      "data": "晴天，"
    },
    {
      "bind_event_id": 118,
      "type": "chunk",
      "event": "section",
      "data": "气温约"
    },
    {
      "bind_event_id": 118,
      "type": "chunk",
      "event": "section",
      "data": "为 25 摄氏"
    },
    {
      "bind_event_id": 118,
      "type": "chunk",
      "event": "section",
      "data": "度"
    },
    {
      "bind_event_id": 118,
      "type": "chunk",
      "event": "section",
      "data": "，"
    },
    {
      "bind_event_id": 118,
      "type": "chunk",
      "event": "section",
      "data": "非"
    },
    {
      "bind_event_id": 118,
      "type": "chunk",
      "event": "section",
      "data": "常宜人。\r\n"
    },
    {
      "bind_event_id": 118,
      "type": "close",
      "event": "section",
      "data": {
        "content": "\r\n根据查询结果，北京现在的天气是晴天，气温约为 25 摄氏度，非常宜人。\r\n",
        "id": 122,
        "tag_name": "reply",
        "time": "1ms 294us 600ns"
      }
    },
    {
      "bind_event_id": 118,
      "type": "open",
      "event": "section",
      "data": "render"
    },
    {
      "bind_event_id": 118,
      "type": "close",
      "event": "section",
      "data": {
        "content": "\r\n{\r\n   \"path\":\"Grant\", \r\n   \"title\":\"获取/usr/info.json文件的创建权限\", \r\n   \"class\":\"flex flex-col\", \r\n   \"style\":\"with:100px;\", \r\n   \"state\":{\r\n    \"appName\":\"Agent\",\r\n    \"reason\":\"需要保存您的个人基本信息以提供个性化服务。\",\r\n    \"duration\":\"30s\"\r\n    }\r\n}\r\n",
        "id": 123,
        "tag_name": "render",
        "time": "3ms 142us 300ns"
      }
    },
    {
      "bind_event_id": 118,
      "type": "end",
      "event": "section",
      "data": {
        "reaction_id": 117,
        "response_id": 118
      }
    }
  ]
