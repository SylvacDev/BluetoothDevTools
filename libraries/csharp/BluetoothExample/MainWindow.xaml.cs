using System.Windows;

namespace BluetoothExample
{
    /// <summary>
    /// Interaction logic for MainWindow.xaml
    /// </summary>
    public partial class MainWindow : Window
    {
        public MainWindow()
        {
            InitializeComponent();
            DataContext = ViewModel = new MainViewModel();
        }

        public MainViewModel ViewModel { get; }
    }
}
