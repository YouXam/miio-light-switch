import React, { Component } from 'react';
import { SafeAreaView } from 'react-navigation';
import { View, Text, Button, TextInput, FlatList, TouchableOpacity, StyleSheet } from 'react-native';
import { Device, Service, DeviceEvent, PackageEvent, Host } from 'miot';
import { DeviceID } from '../modules/consts';
import Navigator from '../modules/navigator';


export default class Bluetooth extends Component {
  state = {
    devices: [],
    deviceName: ''
  };

  BluetoothProps = {
    siid: 7,
    piid: 4
  }

  addDevice = () => {
    if (this.state.deviceName.trim()) {
      const value = {
        value: JSON.stringify([...this.state.devices.map((device) => device.name), this.state.deviceName])
      };
      this.setState((prevState) => ({
        devices: [...prevState.devices, { key: Math.random().toString(), name: prevState.deviceName }],
        deviceName: ''
      }));
      Service.spec.setPropertiesValue([
        Object.assign({ did: DeviceID }, this.BluetoothProps, value)
      ]).then((res) => {
        console.log("update", res);
      }).catch((err) => {
        console.log(err);
      });
    }
  };

  deleteDevice = (key) => {
    const value = {
      value: JSON.stringify(this.state.devices.filter((device) => device.key !== key).map((device) => device.name))
    };
    Service.spec.setPropertiesValue([
      Object.assign({ did: DeviceID }, this.BluetoothProps, value)
    ]).then((res) => {
      console.log("update", res);
    }).catch((err) => {
      console.log(err);
    });
    this.setState((prevState) => ({
      devices: prevState.devices.filter((device) => device.key !== key)
    }));
  };

  updateDevices = (value) => {
    try {
      this.setState({
        devices: JSON.parse(value).map((device) => ({
          key: Math.random().toString(),
          name: device
        }))
      });
    } catch (e) {
      this.setState({
        devices: []
      });
    }
  }

  handleReceivedMessage = (device, message) => {
    const result = message.get(`prop.${ this.BluetoothProps.siid }.${ this.BluetoothProps.piid }`);
    if (result) {
      this.updateDevices(result[0]);
    }
  }

  componentDidMount() {
    this.messageSubscription = DeviceEvent.deviceReceivedMessages.addListener(this.handleReceivedMessage);
    Device.getDeviceWifi().subscribeMessages(`prop.${ this.BluetoothProps.siid }.${ this.BluetoothProps.piid }`)
      .then((subscription) => {
        this.propsSubscription = subscription;
      });
    Service.spec.getPropertiesValue([
      Object.assign({ did: DeviceID }, this.BluetoothProps)
    ], 2).then((res) => {
      this.updateDevices(res[0].value);
    });
  }

  componentWillUnmount() {
    this.messageSubscription && this.messageSubscription.remove();
    this.propsSubscription && this.propsSubscription.remove();
  }

  render() {
    return (
      <SafeAreaView style={styles.safearea}>
        <Navigator navigation={this.props.navigation} title="蓝牙扫描目标设备" hideRightButton={true} />
        <View style={styles.container}>

          <TextInput
            style={styles.input}
            placeholder="设备名称"
            value={this.state.deviceName}
            onChangeText={(text) => this.setState({ deviceName: text })}
          />

          <Button title="添加设备" onPress={this.addDevice} />

          <FlatList
            data={this.state.devices}
            renderItem={({ item }) => (
              <View style={styles.deviceItem}>
                <Text style={styles.deviceText}>{item.name}</Text>
                <TouchableOpacity onPress={() => this.deleteDevice(item.key)}>
                  <Text style={styles.deleteText}>删除</Text>
                </TouchableOpacity>
              </View>
            )}
            keyExtractor={(item) => item.key}
            style={{
              marginTop: 20
            }}
          />
        </View>
      </SafeAreaView>
    );
  }
}

const styles = StyleSheet.create({
  safearea: {
    flex: 1,
    width: '100%'
  },
  container: {
    flex: 1,
    padding: 20,
    backgroundColor: '#f5f5f5'
  },
  backButton: {
    position: 'absolute',
    top: 40,
    left: 20,
    padding: 10,
    backgroundColor: '#007AFF',
    borderRadius: 5
  },
  backButtonText: {
    color: '#fff',
    fontSize: 16
  },
  input: {
    borderWidth: 1,
    padding: 10,
    marginBottom: 20,
    fontSize: 16,
    borderColor: '#ccc',
    borderRadius: 5,
    backgroundColor: '#fff'
  },
  deviceItem: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    padding: 15,
    borderBottomWidth: 1,
    borderColor: '#ddd',
    backgroundColor: '#fff',
    marginBottom: 10,
    borderRadius: 5
  },
  deviceText: {
    fontSize: 18,
    color: '#333'
  },
  deleteText: {
    color: 'red',
    fontSize: 16
  }
});
