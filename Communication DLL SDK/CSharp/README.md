BT5Communication basic use
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
You can get the Main Singleton using
```cs
using SylvacBleLib;

public MainViewModel()
{
     //Init Ble Manager
    _bleManager = BleManager.Instance;
}
```
Then start it with (in this example after the window is loaded)
```cs
private void Loaded()
{
    //Start the BT Communication DLL
    _bleManager.Start();
}
```
And dispose it when exiting the program
```cs
public void OnExit(object sender, EventArgs e)
{
    BleManager.Instance.Dispose();
}
```

## 4. Detect Dongles

The list of compatible Bluetooth dongles is given by `_bleManager.BleCentrals`.

To register to dongle added/removed events
```cs
_bleManager.BleCentralManager.BleCentrals.ListChanged += BleCentrals_ListChanged;

private void BleCentrals_ListChanged(object? sender, LibUtils.Collections.ListMutationEventArgs<SylvacBleLib.Central.BleCentral> e)
{
    foreach (var itemAdded in e.AddedItems)
    {
        Application.Current.Dispatcher.Invoke(new Action(() => { DongleList.Add(itemAdded); }));
    }

    foreach (var itemRemoved in e.RemovedItems)
    {
        Application.Current.Dispatcher.Invoke(new Action(() => { DongleList.Remove(itemRemoved); }));
    }

    SelectedDongle = DongleList.FirstOrDefault();
}
```

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
_bleManager.BleScanManager.StartScan();
```

### Clear the scan list
To clear the scan list:
```cs
_bleManager.BleScanManager.ClearScan();
```
This causes the `BleScanManager.Advertisers.ListChanged` event to be raised.

### Stop scan
To stop the scan:
```cs
_bleManager.BleScanManager.StopScan();
```

## 6. Connect instrument
To be able to connect an instrument you need the dongle object, and the MAC address of the instrument.
The MAC address can be retrieved from the advertising device property `BleAdvItem.Mac`.

Use the command `BleCentral.StartConnect()` to require to connect an instrument. It will generate a `BlePeripheral`
object that allows to interact with the instrument once it is connected.

### Connection
```cs
SelectedDongle.StartConnect(SelectedInstrumentAdvertising.Mac, out BlePeripheral blePeripheral);
```

### Deconnection
```cs
blePeripheral.StartDisconnect();
SelectedDongle.RemovePeripheral(blePeripheral)
```

## 7. Instrument properties
You can access several properties of a connected instrument by simply binding to them in your xaml file.
<!-- Display live value, current unit and measurement mode -->
<TextBlock Text="{Binding NumericalValue}"/>
<TextBlock Text="{Binding MeasurementParams.Unit}"/>
<TextBlock Text="{Binding MeasurementParams.MeasuringMode}"/>

## 8. Send command to instrument
To send command you can use 
```cs
ConnectedInstrument.SendCommand("ID?");
```
To get the response you need to register to the instrument response event first
```cs
_connectedInstrument.InstrumentResponseReceived += _connectedInstrument_InstrumentResponseReceived;

private void _connectedInstrument_InstrumentResponseReceived(object? sender, LibUtils.Event.EventArgsValue<string> e)
{
 //Receive the result of sended commands
}
```
To get the value after pressing on the instrument physical button, you can use the following property
<TextBlock Text="{Binding SmsData}"/>
