import React, { Component } from 'react';
import { StyleSheet, View, Text, Image, Animated, DeviceEventEmitter } from 'react-native';
import { SafeAreaView } from 'react-navigation';
import { Device, Service, DeviceEvent, PackageEvent, Host } from 'miot';
import { LoadingDialog } from 'miot/ui';

import Navigator from '../modules/navigator';
import { LocalizedString, NOOP, PROTOCOLCACHEKEY, DeviceID, SwitchKey, WifiStaCntKey, BluetoothCntKey, IlluminationKey, BluetoothNameKey, adjustSize, getDefinitionWithKeyFromInstance, getInstanceFromNet, formatTimerTime, fixNum } from '../modules/consts';

import Protocol from '../modules/protocol';

import DeviceButton from '../components/device';
import TimingImage from '../components/timing';
import SwitchImage from '../components/switch';
import CountdownImage from '../components/countdown';
import ImageButton from '../components/imageButton';
import TitledImageButton from '../components/titledImageButton';

const IMAGE = {
  light: require("./res/light.png"),
  lightWhite: require("./res/light-white.png"),
  wifi: require("./res/wifi.png"),
  wifiWhite: require("./res/wifi-white.png"),
  bluetooth: require("./res/bluetooth.png"),
  bluetoothWhite: require("./res/bluetooth-white.png")
};

// const DeviceButton = ImageButton(DeviceImage);
const TitledTiming = TitledImageButton(ImageButton(TimingImage));
const TitledSwitch = TitledImageButton(ImageButton(SwitchImage));
const TitledCountdown = TitledImageButton(ImageButton(CountdownImage));

export default class App extends Component {
  constructor(props) {
    super(props);
    this.initProtocol();
  }

  state = {
    on: false,
    timerInfo: '',
    containerBackgroundColor: new Animated.Value(0),
    timingTitle: LocalizedString.setTime(),
    timingActive: false,
    switchTitle: LocalizedString.switch(),
    countdownTitle: LocalizedString.timer(),
    countdownActive: false,
    wifiStaCnt: 0,
    bluetoothCnt: 0,
    Illumination: 0,
    BluetoothName: '',
    BluetoothMatch: false,
    // dialog
    showDialog: false,
    dialogTimeout: 0,
    dialogTitle: ''
  };

  initProtocol = () => {
    Host.storage.get(PROTOCOLCACHEKEY).then((cache) => {
      if (cache) {
        return;
      }
      Host.ui.alertLegalInformationAuthorization(Protocol).then((agreed) => {
        if (agreed) {
          Host.storage.set(PROTOCOLCACHEKEY, true);
        }
      }).catch((_) => { });
    }).catch((_) => { });
  }

  // 记录上次调接口时的值，用以比较并过滤重复操作
  lastModifiedBrightness = 0;
  lastModifiedTemperature = 0;

  switchProp = '';
  wifiStaCntProp = '';
  bluetoothCntProp = '';
  IlluminationKeyProp = '';
  BluetoothNameKeyProp = '';
  BluetoothMatchKeyProp = '';

  SwitchBaseProps = null;
  WifiStaCntBaseProps = null;
  BluetoothCntBaseProps = null;
  IlluminationBaseProps = null;
  BluetoothNameKeyBaseProps = null;
  BluetoothMatchKeyBaseProps = null;

  showLoadingTips = (tip) => {
    return;
  }

  dismissTips = () => {
    this.timerTips && clearTimeout(this.timerTips);
    setTimeout(() => {
      this.setState({
        showDialog: false,
        dialogTimeout: 0,
        dialogTitle: ''
      });
    }, 300);
  }

  showFailTips = (tip) => {
    this.setState({
      showDialog: true,
      dialogTimeout: 300,
      dialogTitle: tip
    });
    this.timerTips && clearTimeout(this.timerTips);
    this.timerTips = setTimeout(() => {
      this.dismissTips();
    }, 300);
  }

  setHandling = (start, end) => {
    this.isHandling = true;
    this.stateAnimation && this.stateAnimation.stop();
    this.stateAnimation = Animated.timing(this.state.containerBackgroundColor, {
      toValue: end,
      duration: 300
    });
    this.state.containerBackgroundColor.setValue(start);
    this.stateAnimation.start((e) => {
      if (e.finished) {
        this.setHandling(end, start);
      }
    });
  }

  setHandled = (on) => {
    this.stateAnimation && this.stateAnimation.stop();
    this.stateAnimation = Animated.timing(this.state.containerBackgroundColor, {
      toValue: on ? 1 : 0,
      duration: 1000
    });
    this.stateAnimation.start(() => {
      this.isHandling = false;
      this.setState({
        on,
        isHandling: false
      });
      this.updateTimerState();
      this.updateNavigationState();
    });
  }

  changeState = (on) => {
    if ((on !== true && on !== false) || on === this.state.on) {
      return;
    }
    this.setState({
      on
    });
    this.updateTimerState();
    this.updateNavigationState();
    this.stateAnimation && this.stateAnimation.stop();
    this.stateAnimation = Animated.timing(this.state.containerBackgroundColor, {
      toValue: on ? 1 : 0,
      duration: 1000
    });
    this.stateAnimation.start();
  }

  switch = () => {
    if (!this.SwitchBaseProps) {
      return;
    }
    // 防止高频提交
    if (this.isHandling) {
      return;
    }
    this.setState({
      isHandling: true
    });
    let on = !this.state.on;
    let switchProps = Object.assign({}, this.SwitchBaseProps, {
      value: on
    });
    this.setHandling(on ? 0 : 0.5, on ? 0.5 : 1);
    // this.showLoadingTips(LocalizedString.handling());
    Service.spec.setPropertiesValue([Object.assign({ did: DeviceID }, switchProps)]).then((_) => {
      let code = _[0].code;
      // 1表示处理中，这里不处理，等消息推送
      if (code === 1) {
        return;
      }
      if (code === 0) {
        this.dismissTips();
        this.setHandled(on);
        return;
      }
      this.setHandled(!on);
      this.showFailTips(LocalizedString.failed());
    }).catch((_) => {
      this.setHandled(!on);
      this.showFailTips(LocalizedString.failed());
    });
  }

  getDeviceProps = (cb) => {
    if (!this.SwitchBaseProps) {
      return;
    }
    Service.spec.getPropertiesValue([
      Object.assign({ did: DeviceID }, this.SwitchBaseProps),
      Object.assign({ did: DeviceID }, this.WifiStaCntBaseProps),
      Object.assign({ did: DeviceID }, this.BluetoothCntBaseProps),
      Object.assign({ did: DeviceID }, this.IlluminationBaseProps),
      Object.assign({ did: DeviceID }, this.BluetoothNameKeyBaseProps),
      Object.assign({ did: DeviceID }, this.BluetoothMatchKeyBaseProps)
    ], 2).then((_) => {
      let formatedProps = this.formatDeviceProps(_);
      this.changeState(formatedProps.on);
      this.setState({
        wifiStaCnt: formatedProps.wifiStaCnt,
        bluetoothCnt: formatedProps.bluetoothCnt,
        Illumination: formatedProps.Illumination,
        BluetoothName: formatedProps.BluetoothName,
        BluetoothMatch: formatedProps.BluetoothMatch
      });
      if (typeof cb === 'function') {
        cb(_);
      }
    }).catch((_) => { });
  }

  formatDeviceProps = (props) => {
    let ret = {};
    for (let prop of props) {
      if (prop.code !== 0) {
        continue;
      }
      let siid = prop.siid,
        piid = prop.piid;
      if (siid === this.SwitchBaseProps.siid && piid === this.SwitchBaseProps.piid) {
        ret.on = prop.value;
      } else if (siid === this.WifiStaCntBaseProps.siid && piid === this.WifiStaCntBaseProps.piid) {
        ret.wifiStaCnt = prop.value;
      } else if (siid === this.BluetoothCntBaseProps.siid && piid === this.BluetoothCntBaseProps.piid) {
        ret.bluetoothCnt = prop.value;
      } else if (siid === this.IlluminationBaseProps.siid && piid === this.IlluminationBaseProps.piid) {
        ret.Illumination = prop.value;
      } else if (siid === this.BluetoothNameKeyBaseProps.siid && piid === this.BluetoothNameKeyBaseProps.piid) {
        ret.BluetoothName = prop.value;
      } else if (siid === this.BluetoothMatchKeyBaseProps.siid && piid === this.BluetoothMatchKeyBaseProps.piid) {
        ret.BluetoothMatch = prop.value;
      }
    }
    return ret;
  }

  updateNavigationState = () => {
    this.props.navigation.setParams({
      barColor: this.state.on ? 'white' : 'black'
    });
  }

  getTimerList = () => {
    Service.scene.loadTimerScenes(DeviceID, {
      identify: DeviceID
    }).then((_) => {
      this.timers = _;
      this.startUpdateTimerState();
    }).catch((_) => { });
  }

  startUpdateTimerState = () => {
    this.updateTimerState();
    this.intervalTimerState && clearInterval(this.intervalTimerState);
    this.intervalTimerState = setInterval(() => {
      this.updateTimerState();
    }, 2e3);
  }

  updateTimerState = () => {
    function getTimerInfo(scene, on) {
      if (!scene) {
        return '';
      }
      let time = scene.time;
      if (scene.timer.setting.timer_type !== '1') {
        return (on ? LocalizedString.timingTipOff : LocalizedString.timingTipOn)(`${ fixNum(time.getHours()) }:${ fixNum(time.getMinutes()) }`);
      }
      let diffMinutes = Math.ceil((time.getTime() - Date.now()) / 1000 / 60);
      let hours = Math.floor(diffMinutes / 60);
      let minutes = diffMinutes - hours * 60;
      return (on ? LocalizedString.countdownTipOff : LocalizedString.countdownTipOn)(hours, minutes);
    }
    let _ = this.timers || [];
    let on = this.state.on;
    let now = new Date();
    let timingScenes = _.filter((item) => {
      return item.setting.enable_timer === '1' && item.setting.timer_type !== '1' && item.status === 0;
    }).map((item) => {
      return {
        timer: item,
        sceneID: item.sceneID,
        time: formatTimerTime(item.setting[on ? 'off_time' : 'on_time'])
      };
    }).filter((item) => {
      return item.time > now;
    });

    let hasTiming = timingScenes.length > 0;

    let countdownScenes = _.filter((item) => {
      // 通过timer_type===1，过滤倒计时
      if (item.setting.enable_timer === '1'
        && (item.setting.timer_type === '1')
        && (item.status === 0)
        && ((item.setting.enable_timer_off === '1' && on)
          || (item.setting.enable_timer_on === '1' && !on))
      ) {
        return true;
      } else {
        return false;
      }
    }).map((item) => {
      return {
        timer: item,
        sceneID: item.sceneID,
        time: formatTimerTime(item.setting[on ? 'off_time' : 'on_time'])
      };
    }).filter((item) => {
      return item.time > now;
    });

    let hasCountdown = countdownScenes.length > 0;

    if (hasCountdown) {
      let recentTimer = countdownScenes.sort((a, b) => {
        return a.time > b.time ? 1 : -1;
      })[0];
      this.firstCountdownTimer = recentTimer;
    } else {
      this.firstCountdownTimer = null;
    }

    let lastScene = (!hasTiming && !hasCountdown) ? null : [...timingScenes, ...countdownScenes].sort((a, b) => {
      return a.time > b.time ? 1 : -1;
    })[0];

    let timerInfo = !lastScene ? '' : getTimerInfo(lastScene, on);
    this.setState({
      timingActive: hasTiming,
      countdownActive: hasCountdown,
      timerInfo
    });
  }

  setTiming = () => {
    if (!this.SwitchBaseProps) {
      return;
    }
    let switchOnProps = Object.assign({}, this.SwitchBaseProps, {
      value: true,
      did: DeviceID
    });
    let switchOffProps = Object.assign({}, this.SwitchBaseProps, {
      value: false,
      did: DeviceID
    });
    Host.ui.openTimerSettingPageWithVariousTypeParams('set_properties', [switchOnProps], 'set_properties', [switchOffProps]);
  }

  setCountdown = () => {
    if (!this.SwitchBaseProps) {
      return;
    }
    let now = new Date();
    let firstCountdownTimer = this.firstCountdownTimer;
    let firstCountdownTime = (this.firstCountdownTimer && this.firstCountdownTimer.time > now) ? this.firstCountdownTimer.time : now;
    let onParam = Object.assign({}, this.SwitchBaseProps, {
      value: true,
      did: DeviceID
    });
    let offParam = Object.assign({}, this.SwitchBaseProps, {
      value: false,
      did: DeviceID
    });
    Service.scene.openCountDownPage(!!this.state.on, {
      onMethod: 'set_properties',
      onParam: [onParam],
      offMethod: 'set_properties',
      offParam: [offParam]
    });
  }

  updateInstance = (instance) => {
    if (!instance) {
      return;
    }
    let defs = getDefinitionWithKeyFromInstance(instance, SwitchKey, WifiStaCntKey, BluetoothCntKey, IlluminationKey, BluetoothNameKey);

    let switchDef = {
      siid: 2,
      piid: 1
    };

    if (switchDef) {
      this.switchProp = `prop.${ switchDef.siid }.${ switchDef.piid }`;
      this.SwitchBaseProps = {
        siid: switchDef.siid,
        piid: switchDef.piid
      };
    }

    let wifiStaCntDef = defs[WifiStaCntKey];
    if (wifiStaCntDef) {
      this.wifiStaCntProp = `prop.${ wifiStaCntDef.siid }.${ wifiStaCntDef.piid }`;
      this.WifiStaCntBaseProps = {
        siid: wifiStaCntDef.siid,
        piid: wifiStaCntDef.piid
      };
    }

    let bluetoothCntDef = defs[BluetoothCntKey];
    if (bluetoothCntDef) {
      this.bluetoothCntProp = `prop.${ bluetoothCntDef.siid }.${ bluetoothCntDef.piid }`;
      this.BluetoothCntBaseProps = {
        siid: bluetoothCntDef.siid,
        piid: bluetoothCntDef.piid
      };
    }

    let IlluminationDef = defs[IlluminationKey];
    if (IlluminationDef) {
      this.IlluminationKeyProp = `prop.${ IlluminationDef.siid }.${ IlluminationDef.piid }`;
      this.IlluminationBaseProps = {
        siid: IlluminationDef.siid,
        piid: IlluminationDef.piid
      };
    }
    let BluetoothNameKeyDef = defs[BluetoothNameKey];
    BluetoothNameKeyDef = {
      siid: 7,
      piid: 4
    };
    if (BluetoothNameKeyDef) {
      this.BluetoothNameKeyProp = `prop.${ BluetoothNameKeyDef.siid }.${ BluetoothNameKeyDef.piid }`;
      this.BluetoothNameKeyBaseProps = {
        siid: BluetoothNameKeyDef.siid,
        piid: BluetoothNameKeyDef.piid
      };
    }

    let BluetoothMatchKeyDef = {
      siid: 7,
      piid: 3
    };

    if (BluetoothMatchKeyDef) {
      this.BluetoothMatchKeyProp = `prop.${ BluetoothMatchKeyDef.siid }.${ BluetoothMatchKeyDef.piid }`;
      this.BluetoothMatchKeyBaseProps = {
        siid: BluetoothMatchKeyDef.siid,
        piid: BluetoothMatchKeyDef.piid
      };
    }

    this.initPropsSubscription();
    this.getDeviceProps(this.getTimerList);
  }

  initPropsSubscription = () => {
    // 状态订阅
    let props = [];
    if (this.switchProp) {
      props.push(this.switchProp);
    }
    if (this.wifiStaCntProp) {
      props.push(this.wifiStaCntProp);
    }
    if (this.bluetoothCntProp) {
      props.push(this.bluetoothCntProp);
    }
    if (this.IlluminationKeyProp) {
      props.push(this.IlluminationKeyProp);
    }
    if (this.BluetoothMatchKeyProp) {
      props.push(this.BluetoothMatchKeyProp);
    }
    if (!props.length) {
      return;
    }
    this.messageSubscription = DeviceEvent.deviceReceivedMessages.addListener(this.handleReceivedMessage);
    Device.getDeviceWifi().subscribeMessages(...props).then((subscription) => {
      this.propsSubscription = subscription;
    }).catch(NOOP);
  }

  handleReceivedMessage = (device, message) => {
    if (!message) {
      return;
    }
    console.log(message);
    this.handleReceivedSwitchMessage(message);
    this.handleReceivedWifiStaCntMessage(message);
    this.handleReceivedBluetoothCntMessage(message);
    this.handleReceivedIlluminationMessage(message);
    this.handleReceivedBluetoothMatchMessage(message);
  }

  handleReceivedSwitchMessage = (message) => {
    if (!message.has(this.switchProp)) {
      return;
    }
    let value = message.get(this.switchProp);
    if (Array.isArray(value)) {
      value = value[0];
    }
    if (typeof value === 'undefined') {
      return;
    }
    // this.changeState(value);
    this.setHandled(value);
  }

  handleReceivedBluetoothMatchMessage = (message) => {
    if (!message.has(this.BluetoothMatchKeyProp)) {
      return;
    }
    let value = message.get(this.BluetoothMatchKeyProp);
    if (Array.isArray(value)) {
      value = value[0];
    }
    if (typeof value === 'undefined') {
      return;
    }
    this.setState({
      BluetoothMatch: value
    });
  }

  handleReceivedWifiStaCntMessage = (message) => {
    if (!message.has(this.wifiStaCntProp)) {
      return;
    }
    let value = message.get(this.wifiStaCntProp);
    if (Array.isArray(value)) {
      value = value[0];
    }
    if (typeof value === 'undefined') {
      return;
    }
    // console.log('wifiStaCnt', value);
    this.setState({
      wifiStaCnt: value
    });
  }

  handleReceivedBluetoothCntMessage = (message) => {
    if (!message.has(this.bluetoothCntProp)) {
      return;
    }
    let value = message.get(this.bluetoothCntProp);
    if (Array.isArray(value)) {
      value = value[0];
    }
    if (typeof value === 'undefined') {
      return;
    }
    this.setState({
      bluetoothCnt: value
    });
    // console.log('bluetoothCnt', value);
  }

  handleReceivedIlluminationMessage = (message) => {
    if (!message.has(this.IlluminationKeyProp)) {
      return;
    }
    let value = message.get(this.IlluminationKeyProp);
    if (Array.isArray(value)) {
      value = value[0];
    }
    if (typeof value === 'undefined') {
      return;
    }
    this.setState({
      Illumination: value
    });
    // console.log('Illumination', value);
  }

  componentDidMount() {
    // 从其他rn页面返回
    this.viewFocusListener && this.viewFocusListener.remove();
    this.viewFocusListener = this.props.navigation.addListener('didFocus', (_) => {
      this.getDeviceProps(this.getTimerList);
    });

    // 从原生页面返回
    this.viewAppearListener && this.viewAppearListener.remove();
    this.viewAppearListener = PackageEvent.packageViewWillAppear.addListener((_) => {
      this.getDeviceProps(this.getTimerList);
    });

    // getInstanceFromCache(this.updateInstance);
    getInstanceFromNet(this.updateInstance);

    this.firmwareChange = DeviceEventEmitter.addListener('MH_FirmwareNeedUpdateAlert', (params) => {
      if (params && params.needUpgrade) {
        this.props.navigation.setParams({
          showDot: true
        });
      }
    });
  }

  componentWillUnmount() {
    this.focusListener && this.focusListener.remove();
    this.viewFocusListener && this.viewFocusListener.remove();
    this.viewAppearListener && this.viewAppearListener.remove();

    this.messageSubscription && this.messageSubscription.remove();
    this.propsSubscription && this.propsSubscription.remove();

    this.rafBrightness && cancelAnimationFrame(this.rafBrightness);
    this.rafTemperature && cancelAnimationFrame(this.rafTemperature);

    this.intervalTimerState && clearInterval(this.intervalTimerState);

    this.firmwareChange && this.firmwareChange.remove();
  }

  render() {
    let {
      on,
      timerInfo,
      timingTitle,
      timingActive,
      countdownTitle,
      countdownActive,
      switchTitle,
      containerBackgroundColor,
      showDialog,
      dialogTimeout,
      dialogTitle,
      wifiStaCnt,
      bluetoothCnt,
      Illumination,
      BluetoothName,
      BluetoothMatch
    } = this.state;

    let deviceTitle = timerInfo || (on ? LocalizedString.powerOn() : LocalizedString.powerOff());

    let deviceProps = {
      on,
      disabled: !!this.isHandling
    };

    let textColor = {
      color: on ? '#fff' : '#000',
      fontSize: adjustSize(16),
      marginTop: adjustSize(5)
    };

    let navigation = this.props.navigation;

    return (
      <Animated.View style={[Styles.container, {
        backgroundColor: containerBackgroundColor.interpolate({
          inputRange: [0, 1],
          outputRange: ['#FAFAFA', '#23BFFF']
        })
      }]}>
        <SafeAreaView style={Styles.safearea}>
          <Navigator navigation={this.props.navigation} />
          <View style={[
            Styles.containerInner
          ]}>
            <View style={Styles.main}>
              <DeviceButton {...deviceProps} title={deviceTitle} onPress={this.switch} />
            </View>
            <View style={Styles.statusBar}>
              <View style={Styles.statusLabel}>
                <Image source={on ? IMAGE.lightWhite : IMAGE.light} style={Styles.icon} />
                <Text style={[Styles.statusName, textColor]}>环境亮度</Text>
              </View>
              <View>
                <Text style={textColor}>{Illumination}</Text>
              </View>
            </View>
            <View style={Styles.statusBar}>
              <View style={Styles.statusLabel}>
                <Image source={on ? IMAGE.wifiWhite : IMAGE.wifi} style={Styles.icon} />
                <Text style={[Styles.statusName, textColor]}>WiFi 同频设备数</Text>
              </View>
              <View>
                <Text style={textColor}>{wifiStaCnt}</Text>
              </View>
            </View>
            <View style={Styles.statusBar}>
              <View style={Styles.statusLabel}>
                <Image source={on ? IMAGE.bluetoothWhite : IMAGE.bluetooth} style={Styles.icon} />
                <Text style={[Styles.statusName, textColor]}>蓝牙设备数</Text>
              </View>
              <View>
                <Text style={textColor}>{bluetoothCnt}</Text>
              </View>
            </View>
            <View style={[
              Styles.statusBar,
              {
                marginBottom: adjustSize(40)
              }
            ]}>
              <View style={Styles.statusLabel}>
                <Image source={on ? IMAGE.bluetoothWhite : IMAGE.bluetooth} style={Styles.icon} />
                <Text style={[Styles.statusName, textColor]}>蓝牙设备扫描</Text>
              </View>
              <View style={{
                flexDirection: "row",
                alignItems: 'center'
              }}>
                <Text
                  style={[
                    textColor,
                    {
                      fontSize: adjustSize(16),
                      borderBottomWidth: 1,
                      borderBottomColor: textColor.color
                    }
                  ]}
                  onPress={() => {
                    navigation.navigate('Bluetooth');
                  }}
                >{
                  BluetoothName?.length !== 0 && BluetoothName !== '[]' ? (
                    BluetoothMatch ? "搜索到目标设备" : "未搜索到目标设备"
                  ) : "未设置目标"
                  }</Text>
              </View>
            </View>
            <View style={[Styles.buttons, Device.isOwner ? null : {
              justifyContent: 'center'
            }]}>
              {Device.isOwner ? (
                <TitledTiming on={on} disabled={!!this.isHandling} active={timingActive} title={timingTitle} onPress={this.setTiming} />
              ) : null}
              <TitledSwitch on={on} disabled={!!this.isHandling} title={switchTitle} onPress={this.switch} />
              {Device.isOwner ? (
                <TitledCountdown on={on} disabled={!!this.isHandling} active={countdownActive} title={countdownTitle} onPress={this.setCountdown} />
              ) : null}
            </View>
          </View>
        </SafeAreaView>
        <LoadingDialog visible={showDialog} message={dialogTitle} timeout={dialogTimeout} />
      </Animated.View>
    );
  }
}

const Styles = StyleSheet.create({
  container: {
    flex: 1,
    justifyContent: 'space-evenly',
    alignItems: 'center'
  },
  safearea: {
    flex: 1,
    width: '100%'
  },
  containerInner: {
    flex: 1,
    width: '100%',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginTop: 0,
    marginBottom: adjustSize(40)
  },
  main: {
    flex: 1,
    justifyContent: 'center'
  },
  sliders: {
    marginTop: adjustSize(-15),
    marginBottom: adjustSize(30)
  },
  sliderWrap: {
    justifyContent: 'center',
    alignItems: 'center',
    flexDirection: 'row',
    marginTop: adjustSize(29)
  },
  sliderIconWrap: {
    width: adjustSize(60),
    alignItems: 'flex-end'
  },
  sliderIcon: {
    width: adjustSize(20),
    height: adjustSize(20)
  },
  slider: {
    width: adjustSize(203),
    height: adjustSize(21),
    // backgroundColor: '#f00',
    marginHorizontal: adjustSize(5)
  },
  sliderText: {
    width: adjustSize(60),
    fontSize: adjustSize(12),
    color: '#ddd',
    fontFamily: 'MI-LANTING--GBK1-Light'
  },
  sliderTextOn: {
    color: '#fff'
  },
  sliderTextOff: {
    color: '#ddd'
  },
  buttons: {
    width: adjustSize(321),
    flexDirection: 'row',
    justifyContent: 'space-between'
  },
  icon: {
    width: adjustSize(24),
    height: adjustSize(24),
    marginTop: adjustSize(5)
  },
  statusName: {
    fontSize: adjustSize(16),
    marginLeft: adjustSize(5)
  },
  statusLabel: {
    flexDirection: 'row',
    alignItems: 'center'
  },
  statusBar: {
    width: adjustSize(321),
    flexDirection: 'row',
    justifyContent: 'space-between',
    marginTop: adjustSize(20)
  }
});
