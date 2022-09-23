![MIT](https://img.shields.io/badge/license-MIT-green)
[![CI](https://github.com/litcc/tts-server/actions/workflows/rust.yml/badge.svg)](https://github.com/litcc/tts-server/actions/workflows/rust.yml)
![GitHub release (latest by date)](https://img.shields.io/github/downloads/litcc/tts-server/latest/total)

# tts-server

- 本项目目前使用的是 Edge 浏览器“大声朗读”和 Azure TTS 演示页面的接口，以及官方订阅接口，除官方订阅接口外，不保证后续可用性和稳定性。还是强烈推荐使用订阅接口！

- 项目使用保持连接的 websocket 极大提升了请求并发性以及请求速度，减少了频繁使用 http 到 websocket 升级协议握手的时间（如果国内服务器的话可能不太明显，国外服务器的情况下，重连很耗费时间）


- 代码可能会有点乱，介意的话请移步下面的相关项目



使用介绍的话没工夫写，可以使用程序里面的 --help ，或者也可以看看 help.md（过期） 文件


如果有人愿意来贡献的话请直接提 PR


且行且珍惜


## 项目动态

2022-09-22: 添加多途径api调用，新增对官方订阅key的使用支持；

2022-08-00: 经反馈 ip 加速失效；

2022-06-16：Edge 浏览器提供的接口现在已经不能设置讲话风格了，简单点说就是 style 参数废了，传什么都默认了；



## 相关项目
排名不分前后
- [ag2s20150909/TTS](https://github.com/ag2s20150909/TTS)：安卓版，可代替系统自带的TTS。
- [wxxxcxx/ms-ra-forwarder](https://github.com/wxxxcxx/ms-ra-forwarder)：Nodejs 运行的版本，自带web页面。
- [jing332/tts-server-go](https://github.com/jing332/tts-server-go)：Go语言实现版本。
- [jing332/tts-server-android](https://github.com/jing332/tts-server-android)：tts-server-go 的 Android 实现版本。



## 免责声明

微软官方的 Azure TTS 服务目前拥有一定的免费额度，如果免费额度对你来说够用的话，请支持官方的服务。

如果只需要为固定的文本生成语音，可以使用有声内容创作。它提供了更丰富的功能可以生成更自然的声音。

本项目构建的二进制程序仅供学习交流和参考，严禁用于商业用途，请于下载后24小时内删除！