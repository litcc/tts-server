# Windows端：

##### 切换到 tts-server.exe所在路径

##### shell输入以下命令

` ./tts-server.exe --help`

##### cmd输入

`tts-server.exe --help`

##### 会发现出来一堆文本

```
tts-server 0.1.2

litcc

TTS Api Server 软件仅供学习交流，严禁用于商业用途，请于24小时内删除！
目前已实现接口有：[微软文本转语音] 后续看情况可能会再加入其他接口。

微软文本转语音接口： /tts-ms
接口支持 get,post 请求, get请求时参数拼接在url上,使			用post时,参数请使用json body传递。
目前支持参数有:
text - 待转换内容 必填参数
informant - 发音人 可选参数,大小写严格, 默认为 zh-CN-XiaoxiaoNeural

可通过命令行参数查看所有支持的列表
style - 发音风格 可选参数，默认为 general
rate - 语速 可选参数 值范围 0-3 可保留两位小数, 默认为 1
pitch - 音调 可选参数 值范围 0-2 可保留两位小数, 默认为 1
quality - 音频格式 可选参数,默认为 audio-24khz-	48kbitrate-mono-mp3
可通过命令行参数查看所有支持的列表

基本使用教程:
举例： 在开源软件[阅读]App中可以使用如下配置来使用该接口
http://ip:port/tts-ms,{
"method": "POST",
"body": {
"informant": "zh-CN-XiaoxiaoNeural",
"style": "general",
"rate": {{ speakSpeed / 15 }},
"text": "{{java.encodeURI(speakText)}}"
}
}



USAGE:
tts-server.exe [OPTIONS]

OPTIONS:
--debug
是否开启 debug 日志

--do-not-update-speakers-list
指定不从官方更新最新发音人 (可以快速使用本地缓存启动程序)

-h, --help
Print help information

--listen-address <address>
监听地址

[default: 0.0.0.0]

--listen-port <prot>
监听端口

[default: 8080]
--log-path <LOG_PATH>
日志文件路径

[default: C:\Users\ilhsy\AppData\Local\Temp\/local_ocr/ocr.log]

--log-to-file
将日志记录至文件

--server-area <area>
指定连接渠道

[default: default]
[possible values: default, china, china-hk, china-tw]


--show-informant-list
显示可用发音人列表

--show-quality-list
显示音频质量参数列表
-V, --version
Print version information
```




##### 找到一般需要使用的
`        --listen-address <address>
            监听地址

            [default: 0.0.0.0]

        --listen-port <prot>
            监听端口

            [default: 8080]
`

###### 执行命令
`./tts-server.exe --listen-address 192.168.0.101 --listen-port 20222`

192.168.0.101  是指本地IP    20222是监听端口

当然也可以直接双击tts-server.exe，就直接是默认的端口：8080

shell/cmd 不能关闭，否则程序断开

# Linux端

###### 与Windows端一致，不过可使用screen等放在后台，同样可直接./tts-server，IP会是本机IP，端口为8080

###### 输入

`./tts-server.exe --help`

`./tts-server.exe --listen-address 192.168.0.101 --listen-port 20222`

本机IP192.168.0.101需自行调整，监听端口20222看个人喜好

# 阅读导入

```
http://192.168.0.101:20222/tts-ms,{
                    "method": "POST",
                    "body": {
                        "informant": "zh-CN-XiaochenNeural",
                        "style": "general",
                        "rate": {{ speakSpeed / 6.5 }},
                        "quality":"audio-48khz-96kbitrate-mono-mp3",
                        "text": "{{java.encodeURI(speakText)}}"
                    }
                }
```

###### 根据以上模板修改IP、端口

发音人 "informant" 、风格 "style" 、朗读语速 "rate" 与音频格式 "quality" 可根据自己喜好修改
以及`--help`所提及的音调 “pitch”

如果音频格式选择默认可删除这一行
` "quality":"audio-48khz-96kbitrate-mono-mp3",`

以上结束，本人菜鸡，瞎写的帮助文档，不过程序能够运行就是了👀️ 


