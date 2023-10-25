Sylvac Bluetooth library basic use
=========================================

## 1. Introduction
This document shows how to use the `SylvacBleLib` Bluetooth library. This library aims at connecting Sylvac Bluetooth instruments using the *Silabs Bluegiga BLED112* Bluetooth 4 dongle or the *Laird Connectivity (Zephyr)* Bluetooth 5 dongle.

`SylvacBleLib` is still in beta version and the API may change without further notice.

## 2. Install

Put the `SylvacBleLib` folder found in the zip archive of the SDK at the root of your solution.

You need to reference all DLLs in the `SylvacBleLib` folder (found in the SDK), except the `dllres` folder.

Add the following post-build event to your project :

`xcopy "$(SolutionDir)SylvacBleLib\dllres\win-x64\serialwin.dll" "$(TargetDir)dllres\win-x64\" /Y /I`

Install the following NuGet package : `System.IO.Ports` by Microsoft.

## 3. Initialization of the DLL
You can get the Bluetooth manager via the `SylvacBleLib.BleManager.Instance` singleton.
```cs
using SylvacBleLib;

_bleManager = BleManager.Instance;
```
Then start Bluetooth functionality with the `Start()` method of `BleManager`
```cs
//Start the BT Communication DLL
_bleManager.Start();
```
And dispose it when exiting the program
```cs
public void OnExit(object sender, EventArgs e)
{
    BleManager.Instance.Dispose();
}
```

## 4. Detect Dongles

The list of compatible Bluetooth dongles is given by `BleManager.Instance.BleCentrals`.

To register to dongle added/removed events, use the `ListChanged` event:
```cs
_bleManager.BleCentrals.ListChanged += BleCentrals_ListChanged;

private void BleCentrals_ListChanged(object? sender, ListMutationEventArgs<BleCentral> e)
{
    foreach (var itemAdded in e.AddedItems)
    {
        // Update the ObservableCollection on the UI thread.
        Application.Current.Dispatcher.Invoke(() => DongleList.Add(itemAdded));
    }

    foreach (var itemRemoved in e.RemovedItems)
    {
        // Update the ObservableCollection on the UI thread.
        Application.Current.Dispatcher.Invoke(() => DongleList.Remove(itemRemoved));
    }

    SelectedDongle = DongleList.FirstOrDefault();
}
```

### 4.1 Note on the received events thread
The events received from `BleManager` are on a non-UI thread used by the manager to transmit events.

This thread should not be blocked (using `lock` or mutexes for example), otherwise no other event will be received from `BleManager` until the thread is unblocked.

Since the `BleManager` is not a UI thread, it is necessary to use a dispatcher to interact with the UI. For example, `Application.Current.Dispatcher.Invoke` in WPF.

## 5. Scan instruments

The list of advertising devices (*scan list*) is given by `_bleManager.BleScanManager.Advertisers`.

To register to advertising scan events, subscribe to the `Advertisers.ListChanged` event:
```cs
_bleManager.BleScanManager.Advertisers.ListChanged += Advertisers_ListChanged;
```

Then the event handler may look like:
```cs
private void Advertisers_ListChanged(object? sender, LibUtils.Collections.ListMutationEventArgs<SylvacBleLib.Scanning.BleAdvItem> e)
{
    foreach (var itemAdded in e.AddedItems)
    {
        //Filter only Sylvac instrument
        if (itemAdded.AdvertismentName.Contains("SY"))
        {
            Application.Current.Dispatcher.Invoke(new Action(() => { InstrumentAdvertisingList.Add(itemAdded); }));
        }
    }

    foreach (var itemRemoved in e.RemovedItems)
    {
        Application.Current.Dispatcher.Invoke(new Action(() => { InstrumentAdvertisingList.Remove(itemRemoved); }));
    }
}
```
Then start the Scan
```cs
//Start the Scan
BleManager.Instance.BleScanManager.StartScan();
```

### Clear the scan list
To clear the scan list:
```cs
BleManager.Instance.BleScanManager.ClearScan();
```
This causes the `BleScanManager.Advertisers.ListChanged` event to be raised.

### Stop scan
To stop the scan:
```cs
BleManager.Instance.BleScanManager.StopScan();
```

## 6. Connect instrument
To be able to connect an instrument you need the dongle object, and the MAC address of the instrument.
The MAC address can be retrieved from the advertising device property `BleAdvItem.Mac`.

Use the method `BleCentral.StartConnect()` to request a connection to an instrument. It will generate a `BlePeripheral`
object that allows to interact with the instrument once it is connected.

### Connection
```cs
SelectedDongle.StartConnect(bleAdvItem.Mac, out BlePeripheral blePeripheral);
```

### Disconnection
Disconnect the peripheral but keep it assigned to the dongle:
```cs
blePeripheral.StartDisconnect();
```
Or disconnect the peripheral and unassign it from the dongle:
```cs
SelectedDongle.RemovePeripheral(blePeripheral)
```

## 7. Instrument properties
The live measurement and other information from the instrument can retrieved with the following properties.

- `BlePeripheral.NumericalValue` gets the current floating point value.
  - `BlePeripheral.NumericalValueChanged` to receive every value in an event.
  - `BlePeripheral.PropertyChanged` to be notified of property changes.
- `BlePeripheral.MeasurementParams.Unit` gets the current unit.
- `BlePeripheral.MeasurementParams.MeasuringMode` gets the current measuring mode (normal, min, max, delta).
- `BlePeripheral.MeasurementParams.PropertyChanged` to be notified of property changes.

## 8. Send a command to an instrument
To send a command to an instrument, you can use the `SendCommand()` method of `BlePeripheral`.
```cs
blePeripheral.SendCommand("ID?");
```

### 8.1 Receive a response from an instrument
To get the response of the instrument, you can register to the `BlePeripheral.InstrumentResponseReceived` event.

```cs
blePeripheral.InstrumentResponseReceived += BlePeripheral_InstrumentResponseReceived;

private void BlePeripheral_InstrumentResponseReceived(object? sender, LibUtils.Event.EventArgsValue<string> e)
{
    // Receive the response to the sent command.
}
```

## 9. Receive a value sent with the instrument button
To get the value sent by pressing on the instrument physical button, you can register to the `BlePeripheral.InstrumentButtonPressed` event.

```cs
blePeripheral.InstrumentButtonPressed += BlePeripheral_InstrumentButtonPressed;

private void BlePeripheral_InstrumentButtonPressed(object? sender, LibUtils.Event.EventArgsValue<string> e)
{
    // Receive value sent with the instrument button.
}
```
