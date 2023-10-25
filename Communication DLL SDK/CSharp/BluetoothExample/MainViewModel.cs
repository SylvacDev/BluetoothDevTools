using CoreWpfUtils.Commands;
using LibUtils.Collections;
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
        public event PropertyChangedEventHandler? PropertyChanged;

        public BleManager _bleManager = BleManager.Instance;

        public ObservableCollection<BleCentral> DongleList { get; }
        public ObservableCollection<BleAdvItem> InstrumentAdvertisingList { get; }
        public ObservableCollection<BlePeripheral> ConnectedInstrumentsList { get; }

        public SimpleCommand StartScanCommand { get; }
        public SimpleCommand StopScanCommand { get; }
        public SimpleCommand ClearScanCommand { get; }
        public SimpleCommand ConnectInstrumentCommand { get; }
        public SimpleCommand DisconnectInstrumentCommand { get; }

        public MainViewModel()
        {
            _bleManager = BleManager.Instance;

            // Start the Bluetooth manager.
            _bleManager.Start();

            _bleManager.BleCentrals.ListChanged += BleCentrals_ListChanged;
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
                    NotifyPropertyChanged(nameof(SelectedDongle));
                }
            }
        }
        private BleCentral? _selectedDongle;

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

        private void Advertisers_ListChanged(object? sender, ListMutationEventArgs<BleAdvItem> e)
        {
            foreach (var itemAdded in e.AddedItems)
            {
                // Filter to keep only Sylvac instruments.
                if (itemAdded.AdvertismentName.Contains("SY"))
                {
                    Application.Current.Dispatcher.Invoke(() => InstrumentAdvertisingList.Add(itemAdded));
                }
            }

            foreach (var itemRemoved in e.RemovedItems)
            {
                Application.Current.Dispatcher.Invoke(() => InstrumentAdvertisingList.Remove(itemRemoved));
            }
        }

        private void Peripherals_ListChanged(object? sender, ListMutationEventArgs<BlePeripheral> e)
        {
            foreach (var itemAdded in e.AddedItems)
            {
                Application.Current.Dispatcher.Invoke(() => ConnectedInstrumentsList.Add(itemAdded));
            }

            foreach (var itemRemoved in e.RemovedItems)
            {
                Application.Current.Dispatcher.Invoke(() => ConnectedInstrumentsList.Remove(itemRemoved));
            }
        }

        private void StartScan()
        {
            // Filter to receive only Sylvac-compatible advertisements.
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

        protected void NotifyPropertyChanged(string propertyName)
        {
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
        }
    }
}
