#include <QDebug>
#include <QCoreApplication>
#include <QFile>

#include <QtCore>

void add(QFile* file, QString name, int value) {
  file->write(QString("#define QTCW_sizeof_%1 %2\n").arg(name).arg(value).toLatin1());
}

#define ADD(name) add(&file, #name, sizeof(name))

int main(int argc, char *argv[]) {
  QCoreApplication app(argc, argv);
  QStringList args = app.arguments();
  if (args.count() < 2) {
    qFatal("size_definer: no filename supplied.");
    return 1;
  }
  qDebug() << "size_definer: Generating file: " << args[1];
  QFile file(args[1]);
  if (!file.open(QFile::WriteOnly)) {
    qFatal("size_definer: can't open file.");
    return 2;
  }
  //file.write(QString("#define QTCW_sizeof_%1 %2\n").arg("QRect").arg(sizeof(QRect)).toLatin1());
  ADD(QPoint);
  ADD(QRect);

  return 0;
}