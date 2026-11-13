"""Writes the fake vendor packages for the tests.

The zips copy the folder layout of the real AWS, Google Cloud, and Azure
packages. The icons are simple shapes, not vendor files. Run this script
from this folder: python3 make_fake.py
"""
import zipfile


def svg(color, shape):
    body = {
        'circle': '<circle cx="20" cy="20" r="9" fill="none" stroke="#fff" stroke-width="3"/>',
        'tri': '<path d="M20 10l10 18H10z" fill="#fff"/>',
        'bars': '<path d="M11 12h18v4H11zM11 18h18v4H11zM11 24h18v4H11z" fill="#fff"/>',
        'dot': '<circle cx="20" cy="20" r="5" fill="#fff"/>',
        'diamond': '<path d="M20 9l11 11-11 11-11-11z" fill="#fff"/>',
    }[shape]
    return (f'<svg xmlns="http://www.w3.org/2000/svg" width="40" height="40" viewBox="0 0 40 40">'
            f'<rect width="40" height="40" fill="{color}"/>{body}</svg>\n')


def mark(color, shape):
    """A mark with no square of its own, as a resource or product icon has."""
    return svg(color, shape).replace(f'<rect width="40" height="40" fill="{color}"/>', '').replace('#fff', color)


def write(name, files):
    with zipfile.ZipFile(name, 'w') as z:
        for i, (path, body) in enumerate(files):
            info = zipfile.ZipInfo(path, date_time=(2026, 1, 1, 0, 0, 0))
            # The methods alternate, so the tests read both.
            info.compress_type = zipfile.ZIP_DEFLATED if i % 2 == 0 else zipfile.ZIP_STORED
            z.writestr(info, body)


A = 'Architecture-Service-Icons_01012026/'
G = 'Architecture-Group-Icons_01012026/'
R = 'Resource-Icons_01012026/'
write('aws.zip', [
    (A + 'Arch_Compute/64/Arch_Amazon-EC2_64.svg', svg('#ed7100', 'bars')),
    (A + 'Arch_Compute/32/Arch_Amazon-EC2_32.svg', svg('#000000', 'bars')),
    (A + 'Arch_Compute/64/Arch_Amazon-EC2_64.png', 'not svg'),
    ('__MACOSX/' + A + 'Arch_Compute/64/._Arch_Amazon-EC2_64.svg', 'junk'),
    (A + 'Arch_Compute/64/Arch_AWS-Lambda_64.svg', svg('#ed7100', 'tri')),
    (A + 'Arch_Storage/64/Arch_Amazon-Simple-Storage-Service_64.svg', svg('#7aa116', 'circle')),
    (A + 'Arch_Databases/64/Arch_Amazon-RDS_64.svg', svg('#c925d1', 'bars')),
    (A + 'Arch_App-Integration/64/Arch_Amazon-Simple-Queue-Service_64.svg', svg('#e7157b', 'dot')),
    (A + 'Arch_Networking/64/Arch_Amazon-CloudFront_64.svg', svg('#8c4fff', 'circle')),
    (R + 'Res_Networking/Res_Elastic-Load-Balancing_Application-Load-Balancer_48.svg', mark('#8c4fff', 'diamond')),
    (G + 'AWS-Cloud-logo_32.svg', svg('#232f3e', 'dot')),
    (G + 'AWS-Cloud-logo_32_Dark.svg', svg('#ffffff', 'dot')),
    (G + 'Region_32.svg', svg('#00a4a6', 'tri')),
    (G + 'Virtual-private-cloud-VPC_32.svg', svg('#8c4fff', 'dot')),
    (G + 'Public-subnet_32.svg', svg('#7aa116', 'dot')),
    (G + 'Private-subnet_32.svg', svg('#00a4a6', 'dot')),
])
write('gcp-core.zip', [
    ('Unique Icons/BigQuery/SVG/BigQuery-512-color.svg', mark('#4285f4', 'circle')),
    ('Unique Icons/BigQuery/SVG/.DS_Store', 'junk'),
    ('Unique Icons/Cloud Storage/SVG/Cloud_Storage-512-color.svg', mark('#34a853', 'bars')),
    ('Unique Icons/Cloud Run/SVG/CloudRun-512-color-rgb.svg', mark('#4285f4', 'tri')),
    ('Unique Icons/GKE/SVG/GKE-512-color.svg', mark('#4285f4', 'diamond')),
    ('Unique Icons/GKE/PNG/GKE-512-color.png', 'not svg'),
])
write('gcp-category.zip', [
    ('Category Icons/Integration Services/SVG/IntegrationServices-512-color.svg', mark('#ea4335', 'dot')),
    ('Category Icons/Databases/SVG/Databases-512-color.svg', mark('#fbbc04', 'bars')),
    ('Category Icons/Storage/SVG/Storage-512-color.svg', mark('#34a853', 'circle')),
    ('Category Icons/AI _ Machine Learning/SVG/AIMachineLearning-512-color.svg', mark('#4285f4', 'dot')),
])
P = 'Azure_Public_Service_Icons/Icons/'
write('azure.zip', [
    ('Azure_Public_Service_Icons/Azure_Icons_FAQ.pdf', 'pdf'),
    (P + 'compute/10021-icon-service-Virtual-Machine.svg', mark('#0078d4', 'bars')),
    (P + 'containers/10023-icon-service-Kubernetes-Services.svg', mark('#773adc', 'diamond')),
    (P + 'databases/10121-icon-service-Azure-Cosmos-DB.svg', mark('#0078d4', 'circle')),
    (P + 'iot/10121-icon-service-Azure-Cosmos-DB.svg', mark('#ff0000', 'circle')),
    (P + 'networking/10061-icon-service-Virtual-Networks.svg', mark('#50e6ff', 'tri')),
    (P + 'storage/10086-icon-service-Storage-Accounts.svg', mark('#0078d4', 'dot')),
    (P + 'general/10007-icon-service-Resource-Groups.svg', mark('#0078d4', 'tri')),
])
