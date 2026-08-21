Name:           drm-custom-colorfix
Version:        2.3.0
Release:        1%{?dist}
Summary:        Display color temperature calibration utility for GNOME and COSMIC Wayland desktops

License:        GPL-3.0-or-later
URL:            https://github.com/fus0g/drm-colortemp-custom
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  systemd-rpm-macros
Recommends:     colord

%{?systemd_requires}

%description
drm-custom-colorfix is a tool for adjusting display color temperature and gamma ramps.
It features dual-backend support for GNOME / Wayland (via colord and ICC VCGT profiles)
and direct DRM CRTC control (for COSMIC Desktop and TTY sessions).

%prep
%autosetup -n %{name}-%{version}

%build
cargo build --release

%check
cargo test --release

%install
install -D -p -m 0755 target/release/%{name} %{buildroot}%{_bindir}/%{name}
install -D -p -m 0644 scripts/%{name}.service %{buildroot}%{_unitdir}/%{name}.service
install -D -p -m 0644 %{name}.conf %{buildroot}%{_sysconfdir}/default/%{name}.conf

%post
%systemd_post %{name}.service

%preun
%systemd_preun %{name}.service

%postun
%systemd_postun_with_restart %{name}.service

%files
%license LICENSE
%doc README.md CHANGELOG.md
%{_bindir}/%{name}
%{_unitdir}/%{name}.service
%config(noreplace) %{_sysconfdir}/default/%{name}.conf

%changelog
* Sat Aug 22 2026 fus0g <fus0g@localhost> - 2.3.0-1
- Added native GNOME / Wayland backend via colord and pure-Rust ICC VCGT generation
- Added backend auto-detection and explicit --backend flag
- Maintained 100% compatibility with existing DRM and COSMIC auto-bounce features

* Tue Aug 18 2026 fus0g <fus0g@localhost> - 2.2.0-1
- Streamlined configuration to single constant color temperature and auto-activation
- Removed day/night scheduler and chrono dependency

* Tue Aug 18 2026 fus0g <fus0g@localhost> - 2.1.0-1
- Renamed package to drm-custom-colorfix and added early boot calibration support
