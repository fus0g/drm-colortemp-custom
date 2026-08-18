Name:           drm-custom-colorfix
Version:        2.1.0
Release:        1%{?dist}
Summary:        Screen color temperature and display calibration via DRM for COSMIC desktop

License:        GPL-3.0-or-later
URL:            https://github.com/fus0g/drm-colortemp-custom
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  systemd-rpm-macros

%{?systemd_requires}

%description
drm-custom-colorfix is a tool for adjusting display color temperature and gamma ramps
directly via the Linux Direct Rendering Manager (DRM). It allows managing display
temperature on Wayland compositors (such as COSMIC Desktop) by manipulating
hardware gamma tables at early boot and on demand.

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
* Tue Aug 18 2026 fus0g <fus0g@localhost> - 2.1.0-1
- Renamed package to drm-custom-colorfix and added early boot calibration support
