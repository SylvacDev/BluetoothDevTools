using CoreWpfUtils.Commands;
using SylvacBleLib;
using SylvacBleLib.Central;
using SylvacBleLib.Peripheral;
using SylvacBleLib.Scanning;
using System;
using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Linq;
using System.Windows;

namespace BluetoothExample
{
    public class MainViewModel : INotifyPropertyChanged
    {
#pragma warning disable CS0067
        public event PropertyChangedEventHandler? PropertyChanged;
#pragma warning restore CS0067
        public BleManager _bleManager = BleManager.Instance;
        public ObservableCollection<BleCentral> DongleList { get; }
        public ObservableCollection<BleAdvItem> InstrumentAdvertisingList { get; }
        public ObservableCollection<BlePeripheral> ConnectedInstrumentsList { get; }

        public MainViewModel()
        {
            //Init Ble Manager
            _bleManager = BleManager.Instance;
            _bleManager.Start();

            _bleManager.BleCentralManager.BleCentrals.ListChanged += BleCentrals_ListChanged;
            _bleManager.BleScanManager.Advertisers.ListChanged += Advertisers_ListChanged;
            _bleManager.BlePeripherals.ListChanged += Peripherals_ListChanged;

            DongleList = new ObservableCollection<BleCentral>(_bleManager.BleCentrals);
            InstrumentAdvertisingList = new ObservableCollection<BleAdvItem>(_bleManager.BleScanManager.Advertisers);
            ConnectedInstrumentsList = new ObservableCollection<BlePeripheral>(_bleManager.BlePeripherals);

            StartScanCommand = new SimpleCommand(o => StartScan());
            StopScanCommand = new SimpleCommand(o => StopScan());
            ClearScanCommand = new SimpleCommand(o => ClearScan());
            ConnectInstrumentCommand = new SimpleCommand(ConnectInstrument);
            DisconnectInstrumentCommand = new SimpleCommand(DisconnectInstrument);
        }

        public BleCentral? SelectedDongle
        {
            get { return _selectedDongle; }
            set
            {
                if (_selectedDongle != value)
                {
                    _selectedDongle = value;
                    NotifyPropertyChanged("SelectedDongle");
                }
            }
        }
        private BleCentral? _selectedDongle;

        private void BleCentrals_ListChanged(object? sender, LibUtils.Collections.ListMutationEventArgs<BleCentral> e)
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

        private void Advertisers_ListChanged(object? sender, LibUtils.Collections.ListMutationEventArgs<BleAdvItem> e)
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

        private void Peripherals_ListChanged(object? sender, LibUtils.Collections.ListMutationEventArgs<BlePeripheral> e)
        {
            foreach (var itemAdded in e.AddedItems)
            {
                Application.Current.Dispatcher.Invoke(new Action(() => { ConnectedInstrumentsList.Add(itemAdded); }));
            }

            foreach (var itemRemoved in e.RemovedItems)
            {
                Application.Current.Dispatcher.Invoke(new Action(() => { ConnectedInstrumentsList.Remove(itemRemoved); }));
            }
        }

        public SimpleCommand StartScanCommand { get; }
        public SimpleCommand StopScanCommand { get; }
        public SimpleCommand ClearScanCommand { get; }
        public SimpleCommand ConnectInstrumentCommand { get; }
        public SimpleCommand DisconnectInstrumentCommand { get; }

        private void StartScan()
        {
            _bleManager.BleScanManager.SylvacOnly = true;
            _bleManager.BleScanManager.StartScan();
        }

        private void StopScan()
        {
            _bleManager.BleScanManager.StopScan();
        }

        private void ClearScan()
        {
            _bleManager.BleScanManager.ClearScan();
        }

        private void ConnectInstrument(object? args)
        {
            if (args is BleAdvItem bleAdvItem)
            {
                SelectedDongle?.StartConnect(bleAdvItem.Mac, out _);
            }
        }

        private void DisconnectInstrument(object? args)
        {
            if (args is BlePeripheral blePeripheral)
            {
                blePeripheral.StartDisconnect();
                SelectedDongle?.RemovePeripheral(blePeripheral);
            }
        }

        protected void NotifyPropertyChanged(string info)
        {
            if (PropertyChanged != null)
            {
                PropertyChanged(this, new PropertyChangedEventArgs(info));
            }
        }
    }
}
