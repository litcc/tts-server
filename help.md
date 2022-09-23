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

微软文本转语音接口(edge渠道)： /api/tts-ms-edge
微软文本转语音接口(官网预览渠道)： /api/tts-ms-official-preview
微软文本转语音接口(官方订阅Key渠道)： /api/tts-ms-subscribe-api
接口支持 get,post 请求, get请求时参数拼接在url上,使用post时,参数请使用json body传递。
目前支持参数有:
text - 待转换内容 必填参数
informant - 发音人 可选参数,大小写严格, 默认为 zh-CN-XiaoxiaoNeural

可通过命令行参数查看所有支持的列表,下列参数可能在部分渠道无法使用
style - 发音风格 可选参数，默认为 general
rate - 语速 可选参数 值范围 0-3 可保留两位小数, 默认为 1
pitch - 音调 可选参数 值范围 0-2 可保留两位小数, 默认为 1
quality - 音频格式 可选参数,默认为 audio-24khz-48kbitrate-mono-mp3
可通过命令行参数查看所有支持的列表

基本使用教程:
举例： 在开源软件[阅读]App中可以使用如下配置来使用该接口
    http://ip:port/api/tts-ms-edge,{
        "method": "POST",
        "body": {
            "informant": "zh-CN-XiaoxiaoNeural",
            "style": "general",
            "rate": {{ speakSpeed / 15 }},
            "text": "{{java.encodeURI(speakText).replace('+','%20')}}"
        }
    }

```




##### 找到一般需要使用的
```
--listen-address <address>
监听地址

[default: 0.0.0.0]

--listen-port <prot>
监听端口

[default: 8080]
```

###### 执行命令
`./tts-server.exe --listen-address 192.168.0.101 --listen-port 20222`

192.168.0.101  是指本地IP    20222是监听端口

当然也可以直接双击tts-server.exe，就直接是默认的端口：8080

shell/cmd 不能关闭，否则程序断开

# Linux端

###### 与Windows端一致，不过可使用screen等放在后台，同样可直接 `./tts-server` ，IP会是本机IP，端口为8080

###### 输入

`./tts-server.exe --help`

`./tts-server.exe --listen-address 192.168.0.101 --listen-port 20222`

本机IP192.168.0.101需自行调整，监听端口20222看个人喜好

# 阅读导入

```
    http://192.168.0.101:20222/tts-ms,{
        "method": "POST",
        "body": {
            "informant": "zh-CN-XiaoxiaoNeural",
            "style": "general",
            "rate": {{ speakSpeed / 6.5 }},
            "quality":"audio-48khz-96kbitrate-mono-mp3",
            "text": "{{java.encodeURI(speakText).replace('+','%20')}}"
        }
    }
```

###### 根据以上模板修改IP、端口

发音人 "informant" 、风格 "style" 、朗读语速 "rate" 与音频格式 "quality" 可根据自己喜好修改
以及`--help`所提及的音调 “pitch”

如果音频格式选择默认可删除这一行
` "quality":"audio-48khz-96kbitrate-mono-mp3",`



# 扩展
## 音频格式

#### 输入命令

`./tts-server.exe --show-quality-list`

##### 会发现出来一堆参数,挑选出来排列一下便如下，按需填至"quality" 一栏即可

```
"audio-16khz-128kbitrate-mono-mp3",
"audio-16khz-16bit-32kbps-mono-opus",
"audio-16khz-16kbps-mono-siren",
"audio-16khz-32kbitrate-mono-mp3",
"audio-16khz-64kbitrate-mono-mp3", 
"audio-24khz-160kbitrate-mono-mp3",
"audio-24khz-16bit-24kbps-mono-opus", 
"audio-24khz-16bit-48kbps-mono-opus", 
"audio-24khz-48kbitrate-mono-mp3",
"audio-24khz-96kbitrate-mono-mp3", 
"audio-48khz-192kbitrate-mono-mp3", 
"audio-48khz-96kbitrate-mono-mp3",
"ogg-16khz-16bit-mono-opus",
"ogg-24khz-16bit-mono-opus", 
"ogg-48khz-16bit-mono-opus", 
"raw-16khz-16bit-mono-pcm", 
"raw-16khz-16bit-mono-truesilk", 
"raw-24khz-16bit-mono-pcm", 
"raw-24khz-16bit-mono-truesilk", 
"raw-48khz-16bit-mono-pcm",
"raw-8khz-16bit-mono-pcm", 
"raw-8khz-8bit-mono-alaw", 
"raw-8khz-8bit-mono-mulaw",
"riff-16khz-16bit-mono-pcm", 
"riff-24khz-16bit-mono-pcm",
"riff-48khz-16bit-mono-pcm",
"riff-8khz-16bit-mono-pcm",
"riff-8khz-8bit-mono-alaw", 
"riff-8khz-8bit-mono-mulaw", 
"webm-16khz-16bit-mono-opus",
"webm-24khz-16bit-24kbps-mono-opus", 
"webm-24khz-16bit-mono-opus"
```

## 发音人

```
Xiaoxiao(Neura)-晓晓
Yunyang(Neural)-云扬
Xiaochen(Neural)-晓辰
Xiaohan(Neural)-晓涵
Xiaomo(Neural))-晓墨
Xiaoqiu(Neural)-晓秋
Xiaorui(Neura)-晓睿
Xiaoshuang(Neural)-晓双
Xiaoxuan(Neural)-晓萱
Xiaoyan(Neura)）-晓颜
Xiaoyou(Neural)-晓悠
Yunxi(Neural)-云希
Yunye(Neural)-云野
```

##### 具体可使用命令

`./tts-server.exe --show-informant-list`

及阅读 ~~微软~~ 巨硬官方文档

###### 以下发音风格同理


> https://azure.microsoft.com/zh-cn/services/cognitive-services/text-to-speech/#features

> https://docs.microsoft.com/zh-cn/azure/cognitive-services/speech-service/speech-synthesis-markup?tabs=csharp#adjust-speaking-styles

## 发音风格

##### Style  说明
```
style="affectionate"  以较高的音调和音量表达温暖而亲切的语气。 说话者处于吸引听众注意力的状态。 说话者的个性往往是讨喜的。
style="angry"  表达生气和厌恶的语气。
style="assistant"  以热情而轻松的语气对数字助理讲话。
style="calm"  以沉着冷静的态度说话。 语气、音调和韵律与其他语音类型相比要统一得多。
style="chat"  表达轻松随意的语气。
style="cheerful"  表达积极愉快的语气。
style="customerservice"  以友好热情的语气为客户提供支持。
style="depressed"  调低音调和音量来表达忧郁、沮丧的语气。
style="disgruntled"  表达轻蔑和抱怨的语气。 这种情绪的语音表现出不悦和蔑视。
style="embarrassed"  在说话者感到不舒适时表达不确定、犹豫的语气。
style="empathetic"  表达关心和理解。
style="envious"  当你渴望别人拥有的东西时，表达一种钦佩的语气。
style="fearful"  以较高的音调、较高的音量和较快的语速来表达恐惧、紧张的语气。 说话人处于紧张和不安的状态。
style="gentle"  以较低的音调和音量表达温和、礼貌和愉快的语气。
style="lyrical"  以优美又带感伤的方式表达情感。
style="narration-professional"  以专业、客观的语气朗读内容。
style="narration-relaxed"  为内容阅读表达一种舒缓而悦耳的语气。
style="newscast"  以正式专业的语气叙述新闻。
style="newscast-casual"  以通用、随意的语气发布一般新闻。
style="newscast-formal"  以正式、自信和权威的语气发布新闻。
style="sad"  表达悲伤语气。
style="serious"  表达严肃和命令的语气。 说话者的声音通常比较僵硬，节奏也不那么轻松。
```

###### 大部分发音人无法使用全部风格

具体阅读并使用 ~~微软~~ 巨硬官方文档

> https://azure.microsoft.com/zh-cn/services/cognitive-services/text-to-speech/#features




以上结束，本人菜鸡，瞎写的帮助文档，不过程序能够运行就是了👀️ 
